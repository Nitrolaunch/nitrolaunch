import {
	createEffect,
	createResource,
	createSignal,
	For,
	Match,
	Setter,
	Show,
	Switch,
} from "solid-js";
import InlineSelect from "../../components/input/select/InlineSelect";
import "./PackagesConfig.css";
import { PackageMeta, PackageProperties, PkgRequest } from "../../types";
import { invoke } from "@tauri-apps/api/core";
import {
	parsePkgRequest,
	pkgRequestsEqual,
	pkgRequestToString,
	stringCompare,
} from "../../utils";
import IconButton from "../../components/input/button/IconButton";
import {
	Delete,
	Edit,
	Error,
	Plus,
	Popout,
	Search,
	Trash,
	Upload,
} from "../../icons";
import { errorToast } from "../../components/dialog/Toasts";
import LoadingSpinner from "../../components/utility/LoadingSpinner";
import ResolutionError, {
	ResolutionErrorData,
} from "../../components/package/ResolutionError";
import { Loader } from "../../package";
import IconTextButton from "../../components/input/button/IconTextButton";
import { getBrowseUrl } from "../package/BrowsePackages";
import { canonicalizeListOrSingle } from "../../utils/values";
import DeriveIndicator from "./DeriveIndicator";
import { InstanceConfig, PackageOverrides } from "./read_write";
import Tip from "../../components/dialog/Tip";
import EditableList from "../../components/input/text/EditableList";
import PackageQuickAdd from "../../components/package/PackageQuickAdd";
import { useNavigate } from "@solidjs/router";
import Icon from "../../components/Icon";
import SearchBar from "../../components/input/text/SearchBar";
import Modal from "../../components/dialog/Modal";
import ConfiguredPackageModal from "./ConfiguredPackageModal";
import PackageVersion from "../../components/input/text/PackageVersion";

export default function PackagesConfig(props: PackagesConfigProps) {
	let navigate = useNavigate();

	let [filter, setFilter] = createSignal("user");
	let [sideFilter, setSideFilter] = createSignal("all");
	let [search, setSearch] = createSignal<string | undefined>(undefined);

	let [packageMetas, setPackageMetas] = createSignal<
		{ [key: string]: PackageMeta } | undefined
	>();
	let [packageProps, setPackageProps] = createSignal<
		{ [key: string]: PackageProperties } | undefined
	>();
	let [errors, setErrors] = createSignal<{ [key: string]: string | undefined }>(
		{}
	);
	let [selectedPackage, setSelectedPackage] = createSignal<
		ConfiguredPackageProps | undefined
	>();

	let [allPackages, allPackagesMethods] = createResource(async () => {
		let installedPackages: InstalledPackage[] = [];
		if (!props.isTemplate && props.id != undefined) {
			let map: { [key: string]: LockfilePackage } = await invoke(
				"get_instance_packages",
				{ instance: props.id }
			);
			for (let pkg of Object.keys(map)) {
				let val = map[pkg];
				installedPackages.push({
					pkg: pkg,
					req: parsePkgRequest(pkg),
					contentVersion: val.content_version,
					isInstalled: true,
					// We can set these to defaults since they will be replaced
					isConfigured: false,
					isClient: false,
					isServer: false,
					isDerived: false,
				});
			}
		}

		// Get a list of all packages. We fetch and list all of the packages, and each one is then filtered by checking which groups it is in.
		let allPackages = installedPackages.concat([]);

		function addPackages(
			list: PackageConfig[],
			modifier: (pkg: InstalledPackage) => InstalledPackage
		) {
			for (let config of list) {
				let req = getPackageConfigRequest(config);
				let existingPackage = allPackages.find(
					(x) => x.isInstalled && pkgRequestsEqual(x.req, req)
				);
				allPackages = allPackages.filter(
					(x) => !packageConfigsEqual(x.req, req)
				);

				let pkg: InstalledPackage = {
					pkg: pkgRequestToString(req),
					req: req,
					contentVersion:
						existingPackage == undefined
							? undefined
							: existingPackage.contentVersion,
					config: config,
					isInstalled: existingPackage != undefined,
					isConfigured: true,
					isClient: false,
					isServer: false,
					isDerived: false,
				};

				pkg = modifier(pkg);

				allPackages.push(pkg);
			}
		}

		addPackages(props.derivedGlobalPackages, (x) => {
			x.isDerived = true;
			return x;
		});
		addPackages(props.derivedClientPackages, (x) => {
			x.isDerived = true;
			x.isClient = true;
			return x;
		});
		addPackages(props.derivedServerPackages, (x) => {
			x.isDerived = true;
			x.isServer = true;
			return x;
		});
		addPackages(props.globalPackages, (x) => x);
		addPackages(props.clientPackages, (x) => {
			x.isClient = true;
			return x;
		});
		addPackages(props.serverPackages, (x) => {
			x.isServer = true;
			return x;
		});

		// Get metadata and properties
		let metas: any = {};
		let properties: any = {};
		let errors: any = {};

		try {
			await invoke("preload_packages", {
				packages: allPackages.map((pkg) => pkg.pkg),
				repo: undefined,
			});
		} catch (e) {
			console.error("Failed to preload: " + e);
		}

		let promises = [];
		for (let pkg of allPackages) {
			promises.push(
				(async () => {
					try {
						return [
							pkg.pkg,
							await invoke("get_package_meta_and_props", { package: pkg.pkg }),
						];
					} catch (e) {
						console.error("Failed to load package: " + e);
						errors[pkg.pkg] = e;
						return undefined;
					}
				})()
			);
		}

		let results = await Promise.all(promises);
		for (let result of results) {
			if (result == undefined) {
				continue;
			}

			let [id, [meta, props]] = result as [
				string,
				[PackageMeta, PackageProperties]
			];
			metas[id] = meta;
			properties[id] = props;
		}

		if (Object.keys(errors).length > 0) {
			console.log("One or more packages failed to load");
		}

		setPackageMetas(metas);
		setPackageProps(properties);
		setErrors(errors);

		allPackages.sort((a, b) => stringCompare(a.req.id, b.req.id));

		return allPackages;
	});

	createEffect(() => {
		props.globalPackages;
		props.clientPackages;
		props.serverPackages;
		props.derivedGlobalPackages;
		props.derivedClientPackages;
		props.derivedServerPackages;

		allPackagesMethods.refetch();
	});

	let [resolutionError, resolutionErrorMethods] = createResource(async () => {
		if (props.isTemplate || props.id == undefined) {
			return undefined;
		}

		try {
			let resolutionError: ResolutionErrorData = await invoke(
				"get_instance_resolution_error",
				{ id: props.id }
			);
			return resolutionError;
		} catch (e) {
			console.error("Failed to get resolution error: " + e);
			return undefined;
		}
	});

	let [showQuickAdd, setShowQuickAdd] = createSignal(false);
	let [showOverridesModal, setShowOverridesModal] = createSignal(false);

	return (
		<div class="cont col" id="packages-config">
			<Show when={resolutionError() != undefined}>
				<div class="cont" id="packages-config-resolution-error">
					<ResolutionError error={resolutionError()!} />
				</div>
			</Show>
			<div class="split fullwidth">
				<div class="cont start fullwidth">
					<Tip tip="Add packages" side="top">
						<div style="position:relative">
							<IconButton
								icon={Plus}
								size="1.8rem"
								color="var(--bg2)"
								border="var(--bg3)"
								onClick={() => setShowQuickAdd(!showQuickAdd())}
								shadow
							/>
							<Show when={showQuickAdd()}>
								<div
									class="cont"
									style="position:absolute; top: calc(100% + 1rem);z-index:15"
								>
									<PackageQuickAdd
										onAdd={(pkg) => props.onAdd(pkg, "global")}
										version={props.minecraftVersion}
										loader={props.loader}
									/>
								</div>
							</Show>
						</div>
					</Tip>
					<Tip tip="Browse Packages" side="top">
						<IconButton
							icon={Search}
							size="1.8rem"
							color="var(--bg2)"
							border="var(--bg3)"
							onClick={() => {
								navigate(
									getBrowseUrl(0, undefined, "mod", undefined, {
										minecraft_versions: canonicalizeListOrSingle(
											props.minecraftVersion
										),
										loaders: canonicalizeListOrSingle(props.loader),
										categories: [],
									})
								);
							}}
							shadow
						/>
					</Tip>
					<Tip tip="Edit Manual Overrides" side="top">
						<IconButton
							icon={Edit}
							size="1.8rem"
							color="var(--bg2)"
							border="var(--bg3)"
							onClick={() => {
								setShowOverridesModal(true);
							}}
							shadow
						/>
					</Tip>
				</div>
				<div class="cont end fullwidth">
					<Show when={props.id != undefined && !props.isTemplate}>
						<IconTextButton
							icon={Upload}
							size="1.5rem"
							text="Update Packages"
							onClick={async () => {
								try {
									if (props.beforeUpdate != undefined) {
										await props.beforeUpdate();
									}
									await invoke("update_instance_packages", {
										instanceId: props.id,
									});
								} catch (e) {
									errorToast("Failed to update packages: " + e);
								}
								resolutionErrorMethods.refetch();
							}}
						/>
					</Show>
				</div>
			</div>
			<div></div>
			<div class="fullwidth split3" id="packages-config-header">
				<div class="cont start" id="package-config-filters">
					<InlineSelect
						options={[
							{
								value: "all",
								contents: <div>ALL</div>,
								color: "var(--fg)",
								tip: "All packages",
							},
							{
								value: "user",
								contents: <div>USER</div>,
								color: "var(--instance)",
								tip: "Only packages you have set. No dependencies",
							},
							// {
							// 	value: "bundled",
							// 	contents: <div>BUNDLED</div>,
							// 	color: "var(--package)",
							// 	tip: "Packages from modpacks and bundles",
							// },
							{
								value: "dependencies",
								contents: <div>DEPENDENCIES</div>,
								color: "var(--plugin)",
								tip: "Dependencies of other packages",
							},
						]}
						optionClass="package-config-filter"
						selected={filter()}
						onChange={setFilter}
						grid={false}
						connected={false}
						solidSelect={true}
					/>
				</div>
				<div>
					<SearchBar value={search()} method={setSearch} immediate />
				</div>
				<div class="cont end" id="package-config-sides">
					<Show when={props.isTemplate}>
						<InlineSelect
							options={[
								{ value: "all", contents: <div>ALL</div>, color: "var(--fg)" },
								{
									value: "client",
									contents: <div>CLIENT</div>,
									color: "var(--instance)",
								},
								{
									value: "server",
									contents: <div>SERVER</div>,
									color: "var(--template)",
								},
							]}
							optionClass="package-config-filter"
							selected={sideFilter()}
							onChange={setSideFilter}
							grid={false}
							connected={false}
							solidSelect={true}
						/>
					</Show>
				</div>
			</div>
			<div class="cont col" id="configured-packages">
				<Show
					when={!allPackages.loading}
					fallback={<LoadingSpinner size="5rem" />}
				>
					<For each={allPackages()}>
						{(pkg) => {
							let isVisible = () => {
								if (!pkg.isConfigured && filter() == "user") {
									return false;
								} else if (filter() == "bundled") {
									return false;
								} else if (filter() == "dependencies" && pkg.isConfigured) {
									return false;
								} else if (sideFilter() == "client" && !pkg.isClient) {
									return false;
								} else if (sideFilter() == "server" && !pkg.isServer) {
									return false;
								} else if (
									search() != undefined &&
									!pkg.req.id.includes(search()!) &&
									!(
										meta != undefined &&
										meta.name != undefined &&
										meta.name.includes(search()!)
									)
								) {
									return false;
								}
								return true;
							};

							let meta =
								packageMetas() == undefined
									? undefined
									: packageMetas()![pkg.pkg];
							let properties =
								packageProps() == undefined
									? undefined
									: packageProps()![pkg.pkg];

							return (
								<Show when={isVisible()}>
									<ConfiguredPackage
										pkg={pkg}
										meta={meta}
										props={properties}
										error={errors()[pkg.pkg]}
										onClick={setSelectedPackage}
										onRemove={props.onRemove}
										onVersionChange={(version) => {
											let category: ConfiguredPackageCategory = pkg.isClient
												? "client"
												: pkg.isServer
												? "server"
												: "global";
											let req: PkgRequest = {
												id: pkg.req.id,
												repository: pkg.req.repository,
												version: version,
											};
											console.log("Here");
											props.onAdd(pkgRequestToString(req), category);
										}}
									/>
								</Show>
							);
						}}
					</For>
				</Show>
			</div>
			<ConfiguredPackageModal
				onClose={() => setSelectedPackage(undefined)}
				props={selectedPackage()}
			/>
			<Modal
				visible={showOverridesModal()}
				onClose={setShowOverridesModal}
				width="40rem"
				title="Package Overrides"
				titleIcon={Edit}
				buttons={[
					{
						text: "Close",
						icon: Delete,
						onClick: () => setShowOverridesModal(false),
					},
				]}
			>
				<div class="cont col fullwidth fields">
					<div class="cont start label">
						<label for="launch-env">SUPPRESSED PACKAGES</label>
						<DeriveIndicator
							parentConfigs={props.parentConfigs}
							currentValue={props.overrides.suppress}
							property={(x) =>
								x.overrides == undefined ? undefined : x.overrides.suppress
							}
						/>
					</div>
					<Tip
						tip="These packages will still be installed, but none of their files or relationships will be applied. Perfect for removing or manually replacing packages."
						fullwidth
					>
						<EditableList
							items={canonicalizeListOrSingle(props.overrides.suppress)}
							setItems={(x) => {
								props.setOverrides((overrides) => {
									overrides.suppress = x;
									return overrides;
								});
								props.onChange();
							}}
						/>
					</Tip>
				</div>
			</Modal>
		</div>
	);
}

export interface PackagesConfigProps {
	id?: string;
	globalPackages: PackageConfig[];
	clientPackages: PackageConfig[];
	serverPackages: PackageConfig[];
	derivedGlobalPackages: PackageConfig[];
	derivedClientPackages: PackageConfig[];
	derivedServerPackages: PackageConfig[];
	isTemplate: boolean;
	onRemove: (pkg: string, category: ConfiguredPackageCategory) => void;
	onAdd: (pkg: string, category: ConfiguredPackageCategory) => void;
	setGlobalPackages: (packages: PackageConfig[]) => void;
	setClientPackages: (packages: PackageConfig[]) => void;
	setServerPackages: (packages: PackageConfig[]) => void;
	minecraftVersion?: string;
	loader?: Loader;
	showBrowseButton: boolean;
	parentConfigs: InstanceConfig[];
	onChange: () => void;
	overrides: PackageOverrides;
	setOverrides: Setter<PackageOverrides>;
	beforeUpdate?: () => Promise<void>;
}

function ConfiguredPackage(props: ConfiguredPackageProps) {
	let navigate = useNavigate();

	let [isHovered, setIsHovered] = createSignal(false);
	let name =
		props.meta == undefined || props.meta.name == undefined
			? props.pkg.req.id
			: props.meta.name;

	let icon =
		props.meta == undefined || props.meta.icon == undefined
			? "/icons/default_instance.png"
			: props.meta.icon;

	return (
		<div
			class="shadow bubble-hover-small configured-package"
			onmouseenter={() => setIsHovered(true)}
			onmouseleave={() => setIsHovered(false)}
			onclick={() => props.onClick(props)}
		>
			<div class="cont">
				<Switch>
					<Match when={props.error != undefined}>
						<div class="cont" style="color:var(--error)">
							<Icon icon={Error} size="1.5rem" />
						</div>
					</Match>
					<Match when={props.meta == undefined}>
						<LoadingSpinner size="2rem" />
					</Match>
					<Match when={props.meta != undefined}>
						<img src={icon} class="configured-package-icon" />
					</Match>
				</Switch>
			</div>
			<div class="cont col configured-package-details">
				<div class="cont configured-package-details-top">
					<div class="configured-package-name">{name}</div>
					<PackageVersion
						configuredVersion={props.pkg.req.version}
						installedVersion={props.pkg.contentVersion}
					/>
				</div>
				<Show when={props.pkg.req.repository != undefined}>
					<div class="configured-package-repo">{props.pkg.req.repository}</div>
				</Show>
			</div>
			<div>
				<Show when={props.pkg.isDerived}>
					<div class="cont col fullwidth fullheight">
						<div class="cont configured-package-derive-indicator">DERIVED</div>
					</div>
				</Show>
			</div>
			<div class="cont configured-package-controls">
				<Show when={isHovered()}>
					<IconButton
						icon={Popout}
						size="24px"
						color="var(--bg2)"
						border="var(--bg3)"
						selectedColor="var(--accent)"
						onClick={(e) => {
							e.preventDefault();
							e.stopPropagation();
							navigate(`/packages/package/${props.pkg.pkg}`);
						}}
						selected={false}
					/>
					<Show when={props.pkg.isConfigured && !props.pkg.isDerived}>
						<IconButton
							icon={Trash}
							size="24px"
							color="var(--errorbg)"
							iconColor="var(--error)"
							border="var(--error)"
							selectedColor="var(--accent)"
							onClick={(e) => {
								e.preventDefault();
								e.stopPropagation();
								let category: ConfiguredPackageCategory = props.pkg.isClient
									? "client"
									: props.pkg.isServer
									? "server"
									: "global";
								props.onRemove(props.pkg.pkg, category);
								(e.target! as any).parentElement.parentElement.remove();
							}}
							selected={false}
						/>
					</Show>
				</Show>
			</div>
		</div>
	);
}

export interface ConfiguredPackageProps {
	pkg: InstalledPackage;
	meta?: PackageMeta;
	props?: PackageProperties;
	error?: string;
	onClick: (props: ConfiguredPackageProps) => void;
	onRemove: (pkg: string, category: ConfiguredPackageCategory) => void;
	onVersionChange: (version: string | undefined) => void;
}

export type PackageConfig =
	| string
	| {
			id: string;
	  };

// Gets the PkgRequest from a PackageConfig
export function getPackageConfigRequest(config: PackageConfig) {
	if (typeof config == "string") {
		return parsePkgRequest(config);
	} else {
		return parsePkgRequest(config.id);
	}
}

// Checks if two PackageConfigs are referring to the same package
export function packageConfigsEqual(
	config1: PackageConfig,
	config2: PackageConfig
) {
	let req1 = getPackageConfigRequest(config1);
	let req2 = getPackageConfigRequest(config2);
	return pkgRequestsEqual(req1, req2);
}

// Checks if two PackageConfigs are referring to the same package and version
export function packageConfigsFullyEqual(
	config1: PackageConfig,
	config2: PackageConfig
) {
	let req1 = getPackageConfigRequest(config1);
	let req2 = getPackageConfigRequest(config2);
	return pkgRequestsEqual(req1, req2) && req1.version == req2.version;
}

interface LockfilePackage {
	addons: LockfileAddon[];
	content_version?: string;
}

export interface InstalledPackage {
	pkg: string;
	req: PkgRequest;
	// The currently installed content version
	contentVersion?: string;
	config?: PackageConfig;
	// Whether this package is currently installed
	isInstalled: boolean;
	isConfigured: boolean;
	isClient: boolean;
	isServer: boolean;
	isDerived: boolean;
}

interface LockfileAddon {}

export type ConfiguredPackageCategory = "global" | "client" | "server";
