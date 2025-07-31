import {
	createEffect,
	createMemo,
	createResource,
	createSignal,
	For,
	Setter,
	Show,
} from "solid-js";
import InlineSelect from "../../components/input/InlineSelect";
import "./PackagesConfig.css";
import { PackageMeta, PackageProperties, PkgRequest } from "../../types";
import { invoke } from "@tauri-apps/api";
import {
	parsePkgRequest,
	pkgRequestToString,
	stringCompare,
} from "../../utils";
import IconButton from "../../components/input/IconButton";
import { Delete, Edit, Plus, Popout, Search, Upload } from "../../icons";
import { errorToast } from "../../components/dialog/Toasts";
import LoadingSpinner from "../../components/utility/LoadingSpinner";
import ResolutionError, {
	ResolutionErrorData,
} from "../../components/package/ResolutionError";
import { Loader } from "../../package";
import IconTextButton from "../../components/input/IconTextButton";
import { getBrowseUrl } from "../package/BrowsePackages";
import { canonicalizeListOrSingle } from "../../utils/values";
import Modal from "../../components/dialog/Modal";
import DeriveIndicator from "./DeriveIndicator";
import { InstanceConfig, PackageOverrides } from "./read_write";
import Tip from "../../components/dialog/Tip";
import EditableList from "../../components/input/EditableList";
import PackageQuickAdd from "../../components/package/PackageQuickAdd";

export default function PackagesConfig(props: PackagesConfigProps) {
	let [filter, setFilter] = createSignal("user");
	let [sideFilter, setSideFilter] = createSignal("all");

	let [installedPackages, setInstalledPackages] = createSignal<string[]>([]);

	let [packageMetas, setPackageMetas] = createSignal<
		{ [key: string]: PackageMeta } | undefined
	>();
	let [packageProps, setPackageProps] = createSignal<
		{ [key: string]: PackageProperties } | undefined
	>();

	let [allPackages, allPackagesMethods] = createResource(async () => {
		let installedPackages: string[] = [];
		if (!props.isProfile) {
			let map: { [key: string]: LockfilePackage } = await invoke(
				"get_instance_packages",
				{ instance: props.id }
			);
			installedPackages = installedPackages.concat(Object.keys(map));
		}

		setInstalledPackages(installedPackages);

		// Get a list of all packages. We fetch and list all of the packages, and each one is then filtered by checking which groups it is in.
		let allPackages = installedPackages.concat([]);

		let configsToAdd: PackageConfig[] = [];

		configsToAdd = configsToAdd.concat(props.derivedGlobalPackages);
		configsToAdd = configsToAdd.concat(props.derivedClientPackages);
		configsToAdd = configsToAdd.concat(props.derivedServerPackages);
		configsToAdd = configsToAdd.concat(props.globalPackages);
		configsToAdd = configsToAdd.concat(props.clientPackages);
		configsToAdd = configsToAdd.concat(props.serverPackages);

		for (let pkg of configsToAdd.map(getPackageConfigRequest)) {
			allPackages = allPackages.filter((x) => !packageConfigsEqual(x, pkg));
			allPackages.push(pkgRequestToString(pkg));
		}

		// Get metadata and properties
		let metas: any = {};
		let properties: any = {};

		let promises = [];
		for (let pkg of allPackages) {
			promises.push(
				(async () => {
					try {
						return [
							pkg,
							await invoke("get_package_meta_and_props", { package: pkg }),
						];
					} catch (e) {
						console.error("Failed to load package: " + e);
						return "error";
					}
				})()
			);
		}

		let results = await Promise.all(promises);
		let errorExists = false;
		for (let result of results) {
			if (result == "error") {
				errorExists = true;
				continue;
			}
			let [id, [meta, props]] = result as [
				string,
				[PackageMeta, PackageProperties]
			];
			metas[id] = meta;
			properties[id] = props;
		}

		if (errorExists) {
			errorToast("One or more packages failed to load");
		}

		setPackageMetas(metas);
		setPackageProps(properties);

		allPackages.sort((a, b) =>
			stringCompare(parsePkgRequest(a).id, parsePkgRequest(b).id)
		);

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
		if (props.isProfile) {
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
									onmouseleave={() => setShowQuickAdd(false)}
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
								window.location.href = getBrowseUrl(
									0,
									undefined,
									"mod",
									undefined,
									{
										minecraft_versions: canonicalizeListOrSingle(
											props.minecraftVersion
										),
										loaders: canonicalizeListOrSingle(props.loader),
										categories: [],
									}
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
					<IconTextButton
						icon={Upload}
						size="1.5rem"
						text="Update Packages"
						color="var(--bg2)"
						selectedColor=""
						selected={false}
						onClick={async () => {
							try {
								await invoke("update_instance_packages", {
									instanceId: props.id,
								});
							} catch (e) {
								errorToast("Failed to update packages: " + e);
							}
							resolutionErrorMethods.refetch();
						}}
					/>
				</div>
			</div>
			<div></div>
			<div id="packages-config-header">
				<div
					class="cont"
					id="package-config-filters"
					style="justify-content:flex-start"
				>
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
				<div class="cont" style="justify-content:flex-end">
					<Show when={props.isProfile}>
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
									color: "var(--profile)",
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
					when={installedPackages() != undefined}
					fallback={<LoadingSpinner size="5rem" />}
				>
					<For each={allPackages()}>
						{(pkg) => {
							let derivedGlobalIncludes = createMemo(() =>
								props.derivedGlobalPackages.includes(pkg)
							);
							let derivedClientIncludes = createMemo(() =>
								props.derivedClientPackages.includes(pkg)
							);
							let derivedServerIncludes = createMemo(() =>
								props.derivedServerPackages.includes(pkg)
							);

							let isInstalled = createMemo(() =>
								installedPackages()!.includes(pkg)
							);
							let isClient = createMemo(
								() =>
									props.clientPackages.includes(pkg) || derivedClientIncludes()
							);
							let isServer = createMemo(
								() =>
									props.serverPackages.includes(pkg) || derivedServerIncludes()
							);
							let isConfigured = createMemo(
								() =>
									isClient() ||
									isServer() ||
									props.globalPackages.includes(pkg) ||
									derivedGlobalIncludes()
							);

							let isVisible = () => {
								if (!isConfigured() && filter() == "user") {
									return false;
								} else if (filter() == "bundled") {
									return false;
								} else if (filter() == "dependencies" && isConfigured()) {
									return false;
								} else if (sideFilter() == "client" && !isClient()) {
									return false;
								} else if (sideFilter() == "server" && !isServer()) {
									return false;
								}
								return true;
							};

							let meta =
								packageMetas() == undefined ? undefined : packageMetas()![pkg];
							let properties =
								packageProps() == undefined ? undefined : packageProps()![pkg];

							let isDerived = () =>
								derivedGlobalIncludes() ||
								derivedClientIncludes() ||
								derivedServerIncludes();

							return (
								<Show when={isVisible()}>
									<ConfiguredPackage
										request={parsePkgRequest(pkg)}
										meta={meta}
										props={properties}
										isDerived={isDerived()}
										isInstalled={isInstalled()}
										isConfigured={isConfigured()}
										category={
											isClient() ? "client" : isServer() ? "server" : "global"
										}
										onRemove={props.onRemove}
									/>
								</Show>
							);
						}}
					</For>
				</Show>
			</div>
			<Modal
				visible={showOverridesModal()}
				onClose={setShowOverridesModal}
				width="40rem"
			>
				<div class="cont fullwidth fields" style="padding:2rem">
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
					<div class="cont fullwidth">
						<button
							onclick={() => setShowOverridesModal(false)}
							style="border: 0.15rem solid var(--bg3)"
						>
							Close
						</button>
					</div>
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
	isProfile: boolean;
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
}

function ConfiguredPackage(props: ConfiguredPackageProps) {
	let [isHovered, setIsHovered] = createSignal(false);
	let name =
		props.meta == undefined || props.meta.name == undefined
			? props.request.id
			: props.meta.name;

	let icon =
		props.meta == undefined || props.meta.icon == undefined
			? "/icons/default_instance.png"
			: props.meta.icon;

	return (
		<div
			class="input-shadow configured-package"
			onmouseenter={() => setIsHovered(true)}
			onmouseleave={() => setIsHovered(false)}
		>
			<div class="cont">
				<Show
					when={props.meta != undefined}
					fallback={<LoadingSpinner size="2rem" />}
				>
					<img src={icon} class="configured-package-icon" />
				</Show>
			</div>
			<div class="cont col configured-package-details">
				<div class="cont configured-package-details-top">
					<div class="configured-package-name">{name}</div>
					<Show when={props.request.version != undefined}>
						<div class="configured-package-version">
							{props.request.version}
						</div>
					</Show>
				</div>
				<Show when={props.request.repository != undefined}>
					<div class="configured-package-repo">{props.request.repository}</div>
				</Show>
			</div>
			<div>
				<Show when={props.isDerived}>
					<div class="cont configured-package-derive-indicator">DERIVED</div>
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
							window.location.href = `/packages/package/${pkgRequestToString(
								props.request
							)}`;
						}}
						selected={false}
					/>
					<Show when={props.isConfigured && !props.isDerived}>
						<IconButton
							icon={Delete}
							size="24px"
							color="var(--error)"
							border="var(--error)"
							selectedColor="var(--accent)"
							onClick={(e) => {
								e.preventDefault();
								e.stopPropagation();
								props.onRemove(props.request.id, props.category);
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

interface ConfiguredPackageProps {
	request: PkgRequest;
	meta?: PackageMeta;
	props?: PackageProperties;
	isDerived: boolean;
	isInstalled: boolean;
	isConfigured: boolean;
	category: ConfiguredPackageCategory;
	onRemove: (pkg: string, category: ConfiguredPackageCategory) => void;
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
	return req1.id == req2.id && req1.repository == req2.repository;
}

interface LockfilePackage {
	addons: LockfileAddon[];
}

interface LockfileAddon {}

export type ConfiguredPackageCategory = "global" | "client" | "server";
