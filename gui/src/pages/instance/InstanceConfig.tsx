import { useNavigate } from "@solidjs/router";
import "./InstanceConfig.css";
import { invoke } from "@tauri-apps/api/core";
import {
	createEffect,
	createMemo,
	createResource,
	createSignal,
	Show,
} from "solid-js";
import "@thisbeyond/solid-select/style.css";
import InlineSelect from "../../components/input/select/InlineSelect";
import { clearInputError, inputError } from "../../errors";
import {
	beautifyString,
	getSupportedLoaders,
	parseVersionedString,
} from "../../utils";
import PackagesConfig, {
	PackageConfig,
	packageConfigsEqual,
	packageConfigsFullyEqual,
} from "./PackagesConfig";
import Tip from "../../components/dialog/Tip";
import { errorToast, successToast } from "../../components/dialog/Toasts";
import DisplayShow from "../../components/utility/DisplayShow";
import { getLoaderImage, Loader } from "../../package";
import {
	canonicalizeListOrSingle,
	emptyUndefined,
	undefinedEmpty,
} from "../../utils/values";
import {
	ConfiguredLoaders,
	createConfiguredPackages,
	getConfigPackages,
	getConfiguredLoader,
	getDerivedPackages,
	getDerivedValue,
	getParentTemplates,
	InstanceConfigMode,
	JavaType,
	PackageOverrides,
	parseLaunchMemory,
	readArgs,
	readEditableInstanceConfig,
	saveInstanceConfig,
} from "./read_write";
import { InstanceConfig } from "./read_write";
import DeriveIndicator from "./DeriveIndicator";
import { InstanceInfo } from "../../types";
import Dropdown from "../../components/input/select/Dropdown";
import LoadingSpinner from "../../components/utility/LoadingSpinner";
import LaunchConfig from "./LaunchConfig";
import IconSelector from "../../components/input/select/IconSelector";
import { updateInstanceList } from "./InstanceList";
import SlideSwitch from "../../components/input/SlideSwitch";
import Icon from "../../components/Icon";
import {
	Box,
	Check,
	Controller,
	Delete,
	Gear,
	Play,
	Server,
} from "../../icons";
import LoaderConfig from "./LoaderConfig";
import FloatingTabs from "../../components/input/select/FloatingTabs";
import Modal from "../../components/dialog/Modal";

export default function InstanceConfigModal(props: InstanceConfigProps) {
	let navigate = useNavigate();

	let isInstance = () =>
		props.params == undefined
			? true
			: props.params.mode == InstanceConfigMode.Instance;
	let isTemplate = () =>
		props.params == undefined
			? false
			: props.params.mode == InstanceConfigMode.Template;
	let isBaseTemplate = () =>
		props.params == undefined
			? false
			: props.params.mode == InstanceConfigMode.GlobalTemplate;

	let id = () => (props.params == undefined ? undefined : props.params.id);

	// True is a better default since it expects ID to be undefined
	let isCreating = () =>
		props.params == undefined ? true : props.params.creating;

	let [isDirty, setIsDirty] = createSignal(false);

	let [from, setFrom] = createSignal<string[] | undefined>();

	let [config, configOperations] = createResource(async () => {
		if (props.params == undefined || isCreating()) {
			return undefined;
		}

		// Get the instance or template
		try {
			let configuration = await readEditableInstanceConfig(
				id(),
				props.params.mode
			);
			if (configuration == undefined) {
				errorToast(
					"Could not find instance or template. Please report this issue."
				);
				return undefined;
			}

			setFrom(canonicalizeListOrSingle(configuration.from));
			console.log(configuration);
			return configuration;
		} catch (e) {
			errorToast("Failed to load configuration: " + e);
			return undefined;
		}
	});
	let [parentConfigs, parentConfigOperations] = createResource(
		() => from(),
		async () => {
			if (props.params == undefined) {
				return [];
			} else {
				return await getParentTemplates(from(), props.params.mode);
			}
		},
		{ initialValue: [] }
	);

	createEffect(async () => {
		if (props.params != undefined) {
			setIsDirty(false);

			configOperations.refetch();
			parentConfigOperations.refetch();
			setReleaseVersionsOnly(true);
			setTab("general");

			if (!isBaseTemplate()) {
				try {
					await invoke("set_last_opened_instance", {
						id: id(),
						instanceOrTemplate: props.params.mode,
					});
				} catch (e) {}
			}
		}
	});

	let [releaseVersionsOnly, setReleaseVersionsOnly] = createSignal(true);

	let [supportedMinecraftVersions, supportedVersionsMethods] = createResource(
		async () => {
			let availableVersions = (await invoke("get_minecraft_versions", {
				releasesOnly: releaseVersionsOnly(),
			})) as string[];

			availableVersions.reverse();
			if (releaseVersionsOnly()) {
				return ["latest"].concat(availableVersions);
			} else {
				return ["latest", "latest_snapshot"].concat(availableVersions);
			}
		}
	);

	createEffect(() => {
		releaseVersionsOnly();
		supportedVersionsMethods.refetch();
	});

	let [supportedLoaders, __] = createResource(async () => {
		let loaders = await getSupportedLoaders();
		return [undefined as string | undefined].concat(loaders);
	});

	// Available templates to derive from
	let [templates, ___] = createResource(async () => {
		return (await invoke("get_templates")) as InstanceInfo[];
	});

	let [tab, setTab] = createSignal("general");

	// Input / convenience signals

	// Used to check if we can automatically fill out the ID with the name. We don't want to do this if the user already typed an ID.
	let [isIdDirty, setIsIdDirty] = createSignal(!isCreating());
	let [isTypeDirty, setIsTypeDirty] = createSignal(!isCreating());
	let [isIconDirty, setIsIconDirty] = createSignal(!isCreating());
	let [isVersionDirty, setIsVersionDirty] = createSignal(!isCreating());
	let [isLoaderDirty, setIsLoaderDirty] = createSignal(!isCreating());

	// Config signals
	let [newId, setNewId] = createSignal<string | undefined>();
	let [name, setName] = createSignal<string | undefined>();
	let [side, setSide] = createSignal<"client" | "server" | undefined>();
	let [icon, setIcon] = createSignal<string | undefined>();
	let [version, setVersion] = createSignal<string | undefined>();
	let [clientLoader, setClientLoader] = createSignal<string | undefined>();
	let [clientLoaderVersion, setClientLoaderVersion] = createSignal<
		string | undefined
	>();
	let [serverLoader, setServerLoader] = createSignal<string | undefined>();
	let [serverLoaderVersion, setServerLoaderVersion] = createSignal<
		string | undefined
	>();
	let [datapackFolder, setDatapackFolder] = createSignal<string | undefined>();

	let [globalPackages, setGlobalPackages] = createSignal<PackageConfig[]>([]);
	let [clientPackages, setClientPackages] = createSignal<PackageConfig[]>([]);
	let [serverPackages, setServerPackages] = createSignal<PackageConfig[]>([]);

	let [javaType, setJavaType] = createSignal<JavaType | undefined>(undefined);
	let [initMemory, setInitMemory] = createSignal<number | undefined>(undefined);
	let [maxMemory, setMaxMemory] = createSignal<number | undefined>(undefined);
	let [envVars, setEnvVars] = createSignal<string[]>([]);
	let [jvmArgs, setJvmArgs] = createSignal<string[]>([]);
	let [gameArgs, setGameArgs] = createSignal<string[]>([]);

	let [packageOverrides, setPackageOverrides] = createSignal<PackageOverrides>(
		{}
	);

	let message = () =>
		isInstance()
			? `Instance ${id()}`
			: isBaseTemplate()
			? "Base Template"
			: `Template ${id()}`;

	let derivedPackages = createMemo(() => {
		return getDerivedPackages(parentConfigs());
	});

	// Initialize config signals from config
	createEffect(() => {
		if (config() != undefined) {
			setName(config()!.name);
			setSide(config()!.type);
			setIcon(config()!.icon);
			setVersion(config()!.version);

			// Loader madness
			let [clientLoader, clientLoaderVersion]: [
				string | undefined,
				string | undefined
			] = [undefined, undefined];
			let [serverLoader, serverLoaderVersion]: [
				string | undefined,
				string | undefined
			] = [undefined, undefined];
			let configuredLoader = config()!.loader;
			if (configuredLoader != undefined) {
				if (typeof configuredLoader == "object") {
					if (configuredLoader.client != undefined) {
						[clientLoader, clientLoaderVersion] = parseVersionedString(
							configuredLoader.client
						);
					}
					if (configuredLoader.server != undefined) {
						[serverLoader, serverLoaderVersion] = parseVersionedString(
							configuredLoader.server
						);
					}
				} else {
					let [loader, loaderVersion] = parseVersionedString(configuredLoader);
					clientLoader = loader;
					clientLoaderVersion = loaderVersion;
					serverLoader = loader;
					serverLoaderVersion = loaderVersion;
				}
			}

			setClientLoader(clientLoader);
			setClientLoaderVersion(clientLoaderVersion);
			setServerLoader(serverLoader);
			setServerLoaderVersion(serverLoaderVersion);

			setDatapackFolder(config()!.datapack_folder);

			// Packages
			let [global, client, server] = getConfigPackages(config()!);
			setGlobalPackages(global);
			setClientPackages(client);
			setServerPackages(server);

			// Launch config
			setJavaType(
				config()!.launch == undefined ? undefined : config()!.launch!.java
			);

			let [init, max] = parseLaunchMemory(
				config()!.launch == undefined ? undefined : config()!.launch!.memory
			);
			setInitMemory(init);
			setMaxMemory(max);

			if (config()!.launch == undefined || config()!.launch!.env == undefined) {
				setEnvVars([]);
			} else {
				let out: string[] = [];
				for (let key of Object.keys(config()!.launch!.env!)) {
					out.push(`${key}=${config()!.launch!.env![key]}`);
				}
			}

			if (
				config()!.launch == undefined ||
				config()!.launch!.args == undefined
			) {
				setJvmArgs([]);
				setGameArgs([]);
			} else {
				setJvmArgs(readArgs(config()!.launch!.args?.jvm));
				setGameArgs(readArgs(config()!.launch!.args?.game));
			}

			setPackageOverrides(
				config()!.overrides == undefined ? {} : config()!.overrides!
			);
		}

		// Default side
		if (isCreating() && isInstance()) {
			setSide("client");
		}
	});

	// Writes configuration to disk
	async function saveConfig() {
		if (props.params == undefined) {
			return;
		}

		let configId = isCreating() ? newId() : id();

		if (!isBaseTemplate && configId == undefined) {
			setTab("general");
			inputError("id");
			return;
		} else {
			clearInputError("id");
		}
		if (isCreating()) {
			if (await idExists(configId!, props.params.mode)) {
				setTab("general");
				inputError("id");
				errorToast(
					`${beautifyString(props.params.mode)} with this ID already exists`
				);
				return;
			} else {
				clearInputError("id");
			}
		}

		if (isInstance() && side() == undefined) {
			setTab("general");
			inputError("side");
			return;
		} else {
			clearInputError("side");
		}

		let finalVersion =
			version() == undefined
				? getDerivedValue(parentConfigs(), (x) => x.version)
				: version();
		if (isInstance() && finalVersion == undefined) {
			setTab("general");
			inputError("version");
			return;
		} else {
			clearInputError("version");
		}

		// The user selected custom Java but didn't pick anything
		if (javaType() == "" || javaType() == "custom") {
			setTab("launch");
			inputError("launch-custom-java");
			return;
		} else {
			clearInputError("launch-custom-java");
		}

		// Loaders

		let formatLoader = (loader?: string, version?: string) => {
			if (loader == undefined) {
				return undefined;
			} else {
				return version == undefined ? loader : `${loader}@${version}`;
			}
		};

		let loader = () => {
			if (clientLoader() == undefined && serverLoader() == undefined) {
				return undefined;
			} else {
				if (isInstance()) {
					if (side() == "client") {
						return formatLoader(clientLoader(), clientLoaderVersion());
					} else {
						return formatLoader(serverLoader(), serverLoaderVersion());
					}
				} else {
					if (
						clientLoader() == serverLoader() &&
						clientLoaderVersion() == serverLoaderVersion()
					) {
						return formatLoader(clientLoader(), clientLoaderVersion());
					} else {
						return {
							client: formatLoader(clientLoader(), clientLoaderVersion()),
							server: formatLoader(serverLoader(), serverLoaderVersion()),
						};
					}
				}
			}
		};

		// Packages
		let packages = createConfiguredPackages(
			globalPackages(),
			clientPackages(),
			serverPackages(),
			isInstance()
		);

		// Launch
		let launchMemory =
			initMemory() == undefined || maxMemory() == undefined
				? undefined
				: { min: `${initMemory()}m`, max: `${maxMemory()}m` };

		let env: { [key: string]: string } = {};
		for (let entry of envVars()) {
			let split = entry.split("=");
			if (split.length < 2) {
				continue;
			}

			env[split[0]] = split[1];
		}

		let args =
			jvmArgs() == undefined && gameArgs() == undefined
				? undefined
				: {
						jvm: jvmArgs(),
						game: gameArgs(),
				  };

		let overrides =
			packageOverrides().suppress == undefined ? undefined : packageOverrides();

		let newConfig: InstanceConfig = {
			from: from(),
			type: side(),
			name: undefinedEmpty(name()),
			icon: undefinedEmpty(icon()),
			version: undefinedEmpty(version()),
			loader: loader() as ConfiguredLoaders | undefined,
			packages: packages,
			launch: {
				memory: launchMemory,
				env: Object.keys(env).length == 0 ? undefined : env,
				java: javaType(),
				args: args,
			},
			overrides: overrides,
		};

		// Handle extra fields
		if (config() != undefined) {
			for (let key of Object.keys(config()!)) {
				if (!Object.keys(newConfig).includes(key)) {
					newConfig[key] = config()![key];
				}
			}

			if (config()!.launch != undefined) {
				for (let key of Object.keys(config()!.launch!)) {
					if (!Object.keys(newConfig.launch!).includes(key)) {
						newConfig.launch![key] = config()!.launch![key];
					}
				}
			}
		}

		try {
			await saveInstanceConfig(configId, newConfig, props.params.mode);

			successToast("Changes saved");

			setIsDirty(false);

			updateInstanceList();

			if (isCreating()) {
				navigate("/");
			}

			configOperations.refetch();
			parentConfigOperations.refetch();
		} catch (e) {
			errorToast(e as string);
		}
	}

	let createMessage = () => (isInstance() ? "instance" : "template");

	let saveButtonColor = () =>
		isDirty()
			? isInstance()
				? "var(--instance)"
				: "var(--template)"
			: "var(--bg4)";
	let saveButtonBgColor = () =>
		isDirty()
			? isInstance()
				? "var(--instancebg)"
				: "var(--templatebg)"
			: "var(--bg2)";

	return (
		<Modal
			visible={props.params != undefined}
			onClose={props.onClose}
			width="65rem"
			height="40rem"
			title={
				isCreating()
					? `Creating new ${createMessage()}`
					: `Configure ${message()}`
			}
			titleIcon={Gear}
			buttons={[
				{
					text: "Cancel",
					icon: Delete,
					onClick: props.onClose,
				},
				{
					text: "Save",
					icon: Check,
					onClick: saveConfig,
					color: saveButtonColor(),
					bgColor: saveButtonBgColor(),
				},
			]}
		>
			<div class="cont fullwidth">
				<FloatingTabs
					tabs={[
						{
							id: "general",
							title: "General",
							icon: Gear,
							color: "var(--instance)",
							bgColor: "var(--instancebg)",
						},
						{
							id: "packages",
							title: "Packages",
							icon: Box,
							color: "var(--package)",
							bgColor: "var(--packagebg)",
						},
						{
							id: "launch",
							title: "Launch",
							icon: Play,
							color: "var(--template)",
							bgColor: "var(--templatebg)",
						},
					]}
					selectedTab={tab()}
					setTab={setTab}
				/>
			</div>
			<div></div>
			<DisplayShow when={tab() == "general"} style="width:100%">
				<div id="general-fields-header">
					<div class="cont col fullwidth" style="align-items:flex-start">
						<Show when={!isBaseTemplate()}>
							<div class="cont start label">
								<label for="side">ICON</label>
								<DeriveIndicator
									parentConfigs={parentConfigs()}
									currentValue={icon()}
									property={(x) => x.icon}
								/>
							</div>
							<IconSelector
								icon={icon()}
								setIcon={(x) => {
									setIcon(x);
									setIsDirty(true);
									setIsIconDirty(true);
								}}
								derivedIcon={getDerivedValue(parentConfigs(), (x) => x.icon)}
							/>
						</Show>
					</div>
					<div class="fields">
						<Show when={!isBaseTemplate()}>
							<label for="name" class="label">
								DISPLAY NAME
							</label>
							<Tip
								tip={`The name of the ${beautifyString(props.params!.mode)}`}
								side="top"
								fullwidth
							>
								<input
									type="text"
									id="name"
									name="name"
									placeholder={id()}
									value={emptyUndefined(name())}
									onChange={(e) => {
										setName(e.target.value);
										setIsDirty(true);
									}}
									onKeyUp={(e: any) => {
										// Autofill other fields based on the name
										if (!isIdDirty()) {
											let value = sanitizeInstanceId(e.target.value);
											(document.getElementById("id")! as any).value = value;
											setNewId(value);
										}

										let lowercaseName = e.target.value.toLocaleLowerCase();

										if (!isTypeDirty()) {
											if (lowercaseName.includes("client")) {
												setSide("client");
											} else if (lowercaseName.includes("server")) {
												setSide("server");
											}
										}

										let autofillLoader = undefined;
										if (supportedLoaders() != undefined) {
											for (let loader of supportedLoaders()!) {
												if (
													loader != undefined &&
													lowercaseName.includes(loader)
												) {
													autofillLoader = loader;
													break;
												}
											}
										}
										if (autofillLoader != undefined) {
											if (!isLoaderDirty()) {
												if (side() == "client") {
													setClientLoader(autofillLoader);
												} else if (side() == "server") {
													setServerLoader(autofillLoader);
												}
											}
											if (!isIconDirty()) {
												setIcon(
													"builtin:" + getLoaderImage(autofillLoader as Loader)
												);
											}
										}

										let autofillVersion = undefined;
										if (supportedMinecraftVersions() != undefined) {
											// By going through in forward order, we should catch 1.xx.x before 1.xx
											for (let version of supportedMinecraftVersions()!) {
												if (lowercaseName.includes(version)) {
													autofillVersion = version;
													break;
												}
											}
										}

										if (autofillVersion != undefined && !isVersionDirty()) {
											setVersion(autofillVersion);
										}
									}}
								></input>
							</Tip>
						</Show>
						<Show when={isCreating() && !isBaseTemplate()}>
							<label for="id" class="label">
								ID
							</label>
							<Tip
								tip="A unique name used to identify the instance"
								fullwidth
								side="top"
							>
								<input
									type="text"
									id="id"
									name="id"
									onChange={(e) => {
										setNewId();
										e.target.value = sanitizeInstanceId(e.target.value);
										setNewId(e.target.value);
										setIsDirty(true);
									}}
									onKeyUp={(e: any) => {
										if (
											e.target.value == undefined ||
											e.target.value.length == 0
										) {
											setIsIdDirty(false);
										} else {
											setIsIdDirty(true);
										}
										e.target.value = sanitizeInstanceId(e.target.value);
									}}
								></input>
							</Tip>
						</Show>
					</div>
				</div>
				<br />
				<hr />
				<div class="fields">
					{/* <h3>Basic Settings</h3> */}
					<Show when={!isBaseTemplate()}>
						<div class="cont start label">
							<label for="from">INHERIT CONFIG</label>
						</div>
						<Tip
							tip="A list of templates to inherit configuration from"
							side="top"
							fullwidth
						>
							<Dropdown
								options={
									templates() == undefined
										? []
										: templates()!.map((x) => {
												return {
													value: x.id,
													contents: (
														<div>{x.name == undefined ? x.id : x.name}</div>
													),
													color: "var(--template)",
												};
										  })
								}
								selected={from()}
								onChangeMulti={(x) => {
									setFrom(x);
									setIsDirty(true);
								}}
								isSearchable={false}
								zIndex="50"
							/>
						</Tip>
					</Show>
					<Show
						when={props.params!.creating || isTemplate() || isBaseTemplate()}
					>
						<div class="cont start">
							<label for="side" class="label">
								TYPE
							</label>
							<DeriveIndicator
								parentConfigs={parentConfigs()}
								currentValue={side()}
								property={(x) => x.side}
							/>
						</div>
						<Tip
							tip="Whether this is a normal instance or a dedicated server"
							side="top"
							fullwidth
						>
							<InlineSelect
								onChange={(x) => {
									setSide(x as "client" | "server" | undefined);
									setIsDirty(true);
									setIsTypeDirty(true);
								}}
								selected={side()}
								options={[
									{
										value: "client",
										contents: (
											<div class="cont">
												<Icon icon={Controller} size="1.2rem" /> Client
											</div>
										),
										color: "var(--instance)",
										selectedBgColor: "var(--instancebg)",
									},
									{
										value: "server",
										contents: (
											<div class="cont">
												<Icon icon={Server} size="1rem" /> Server
											</div>
										),
										color: "var(--instance)",
										selectedBgColor: "var(--instancebg)",
									},
								]}
								columns={isInstance() ? 2 : 3}
								allowEmpty={!isInstance()}
							/>
						</Tip>
					</Show>
					<div class="cont start label">
						<label for="version">MINECRAFT VERSION</label>
						<DeriveIndicator
							parentConfigs={parentConfigs()}
							currentValue={version()}
							property={(x) => x.version}
							emptyUndefined
							displayValue
						/>
					</div>
					<Show
						when={supportedMinecraftVersions() != undefined}
						fallback={<LoadingSpinner size="var(--input-height)" />}
					>
						<Tip
							tip="The Minecraft version of this instance"
							fullwidth
							side="top"
						>
							<div class="fullwidth split">
								<div class="fullwidth" id="version">
									<Dropdown
										options={supportedMinecraftVersions()!.map((x) => {
											return {
												value: x,
												contents: (
													<div>
														{x == "latest" || x == "latest_snapshot"
															? beautifyString(x)
															: x}
													</div>
												),
												color: "var(--instance)",
											};
										})}
										selected={version()}
										onChange={(x) => {
											setVersion(x);
											setIsDirty(true);
											setIsVersionDirty(true);
										}}
										allowEmpty
										zIndex="50"
									/>
								</div>
								<div class="cont">
									<SlideSwitch
										enabled={!releaseVersionsOnly()}
										onToggle={() =>
											setReleaseVersionsOnly(!releaseVersionsOnly())
										}
										enabledColor="var(--instance)"
										disabledColor="var(--fg3)"
									/>
									<span
										class="bold"
										style={`color:${
											releaseVersionsOnly() ? "var(--fg3)" : "var(--instance)"
										}`}
									>
										Include Snapshots
									</span>
								</div>
							</div>
						</Tip>
					</Show>
					<LoaderConfig
						minecraftVersion={
							version() == undefined
								? getDerivedValue(parentConfigs(), (x) => x.version)
								: version()
						}
						side={side()}
						isTemplate={isTemplate()}
						clientLoader={clientLoader()}
						serverLoader={serverLoader()}
						clientLoaderVersion={clientLoaderVersion()}
						serverLoaderVersion={serverLoaderVersion()}
						setClientLoader={setClientLoader}
						setServerLoader={setServerLoader}
						setClientLoaderVersion={setClientLoaderVersion}
						setServerLoaderVersion={setServerLoaderVersion}
						supportedLoaders={supportedLoaders()}
						parentConfigs={parentConfigs()}
						setDirty={() => setIsDirty(true)}
						setLoaderDirty={() => setIsLoaderDirty(true)}
					/>
					<hr />
					<div class="cont start label">
						<label for="datapack-folder">DATAPACK FOLDER</label>
						<DeriveIndicator
							parentConfigs={parentConfigs()}
							currentValue={datapackFolder()}
							property={(x) => x.datapack_folder}
							emptyUndefined
						/>
					</div>
					<Tip
						tip="The folder, relative to the instance folder, to put datapacks in. Useful if you have a global datapack mod."
						side="top"
						fullwidth
					>
						<input
							type="text"
							id="datapack-folder"
							name="datapack-folder"
							class="template-placeholder"
							value={emptyUndefined(datapackFolder())}
							onChange={(e) => {
								setDatapackFolder(e.target.value);
								setIsDirty(true);
							}}
							placeholder={getDerivedValue(
								parentConfigs(),
								(x) => x.datapack_folder
							)}
						></input>
					</Tip>
				</div>
				<br />
				<br />
				<br />
			</DisplayShow>
			<DisplayShow when={tab() == "packages"} style="width:100%">
				<PackagesConfig
					id={id()}
					isTemplate={isTemplate()}
					globalPackages={globalPackages()}
					clientPackages={clientPackages()}
					serverPackages={serverPackages()}
					derivedGlobalPackages={derivedPackages()[0]}
					derivedClientPackages={derivedPackages()[1]}
					derivedServerPackages={derivedPackages()[2]}
					onRemove={(pkg, category) => {
						let func = (packages: PackageConfig[]) =>
							packages.filter((x) => !packageConfigsEqual(x, pkg));

						if (category == "global") {
							setGlobalPackages(func);
						} else if (category == "client") {
							setClientPackages(func);
						} else if (category == "server") {
							setServerPackages(func);
						}

						setIsDirty(true);
					}}
					onAdd={(pkg, category) => {
						let func = (packages: PackageConfig[]) => {
							if (!packages.some((x) => packageConfigsFullyEqual(x, pkg))) {
								packages.push(pkg);
								// Force update
								packages = packages.concat([]);
							}
							return packages;
						};

						if (category == "global") {
							setGlobalPackages(func);
						} else if (category == "client") {
							setClientPackages(func);
						} else if (category == "server") {
							setServerPackages(func);
						}

						setIsDirty(true);
					}}
					setGlobalPackages={(packages) => {
						setGlobalPackages(packages);
						setIsDirty(true);
					}}
					setClientPackages={(packages) => {
						setClientPackages(packages);
						setIsDirty(true);
					}}
					setServerPackages={(packages) => {
						setServerPackages(packages);
						setIsDirty(true);
					}}
					minecraftVersion={
						version() == undefined
							? getDerivedValue(parentConfigs(), (x) => x.version)
							: version()
					}
					loader={(() => {
						if (side() == "client") {
							if (clientLoader() == undefined) {
								return getDerivedValue(parentConfigs(), (x) =>
									getConfiguredLoader(x.loader, "client")
								);
							} else {
								return clientLoader();
							}
						} else if (side() == "server") {
							if (serverLoader() == undefined) {
								return getDerivedValue(parentConfigs(), (x) =>
									getConfiguredLoader(x.loader, "server")
								);
							} else {
								return serverLoader();
							}
						} else {
							return undefined;
						}
					})()}
					showBrowseButton={!isCreating()}
					parentConfigs={parentConfigs()}
					onChange={() => setIsDirty(true)}
					overrides={packageOverrides()}
					setOverrides={setPackageOverrides}
					beforeUpdate={saveConfig}
				/>
			</DisplayShow>
			<DisplayShow when={tab() == "launch"} style="width:100%">
				<LaunchConfig
					java={javaType()}
					setJava={setJavaType}
					initMemory={initMemory()}
					maxMemory={maxMemory()}
					setInitMemory={setInitMemory}
					setMaxMemory={setMaxMemory}
					envVars={envVars()}
					setEnvVars={setEnvVars}
					jvmArgs={jvmArgs()}
					gameArgs={gameArgs()}
					setJvmArgs={setJvmArgs}
					setGameArgs={setGameArgs}
					parentConfigs={parentConfigs()}
					onChange={() => setIsDirty(true)}
				/>
			</DisplayShow>
			<br />
			<br />
			<br />
		</Modal>
	);
}

export interface InstanceConfigProps {
	params: InstanceConfigParams | undefined;
	onClose: () => void;
}

// Parameters for the instance config modal
export interface InstanceConfigParams {
	id?: string;
	mode: InstanceConfigMode;
	/* Whether we are creating a new instance or template */
	creating: boolean;
}

// Sanitizes a string so that it is a valid instance ID
export function sanitizeInstanceId(id: string): string {
	id = id.toLocaleLowerCase();
	id = id.replace(/ /g, "-");
	id = id.replace(/\_/g, "-");
	id = id.replace(/\./g, "-");
	// Remove repeated hyphens
	let regex = new RegExp(/-+/, "g");
	id = id.replace(regex, "-");
	// TODO: Sanitize wild characters
	// let regex = new RegExp(/\W/, "ig");
	// id = id.replace(regex, "");
	return id;
}

// Checks if an instance or template ID exists already
async function idExists(
	id: string,
	mode: InstanceConfigMode
): Promise<boolean> {
	let command = `get_${mode}_config`;
	try {
		let result = await invoke(command, { id: id });
		return result != null;
	} catch (e) {
		console.error(e);
		return false;
	}
}
