import { useNavigate, useParams } from "@solidjs/router";
import "./InstanceConfig.css";
import { invoke } from "@tauri-apps/api";
import {
	createEffect,
	createMemo,
	createResource,
	createSignal,
	onMount,
	Show,
} from "solid-js";
import "@thisbeyond/solid-select/style.css";
import InlineSelect from "../../components/input/select/InlineSelect";
import { loadPagePlugins } from "../../plugins";
import { clearInputError, inputError } from "../../errors";
import {
	beautifyString,
	getSupportedLoaders,
	parseVersionedString,
} from "../../utils";
import { FooterData } from "../../App";
import { FooterMode } from "../../components/navigation/Footer";
import PackagesConfig, {
	PackageConfig,
	packageConfigsEqual,
} from "./PackagesConfig";
import Tip from "../../components/dialog/Tip";
import { errorToast, successToast } from "../../components/dialog/Toasts";
import DisplayShow from "../../components/utility/DisplayShow";
import {
	getLoaderColor,
	getLoaderDisplayName,
	getLoaderImage,
	getLoaderSide,
	Loader,
} from "../../package";
import {
	canonicalizeListOrSingle,
	emptyUndefined,
	undefinedEmpty,
} from "../../utils/values";
import {
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
import { Controller, Server } from "../../icons";

export default function InstanceConfigPage(props: InstanceConfigProps) {
	let navigate = useNavigate();

	let params = useParams();

	let isInstance = props.mode == InstanceConfigMode.Instance;
	let isTemplate = props.mode == InstanceConfigMode.Template;
	let isBaseTemplate = props.mode == InstanceConfigMode.GlobalTemplate;

	let id = isInstance
		? params.instanceId
		: isBaseTemplate
			? "Base Template"
			: params.TemplateID;

	onMount(() =>
		loadPagePlugins(
			isInstance
				? "instance_config"
				: isTemplate
					? "template_config"
					: "base_template_config",
			id
		)
	);

	createEffect(async () => {
		props.setFooterData({
			selectedItem: props.creating ? "" : undefined,
			mode: isInstance
				? FooterMode.SaveInstanceConfig
				: FooterMode.SaveTemplateConfig,
			action: saveConfig,
		});

		try {
			await invoke("set_last_opened_instance", {
				id: id,
				instanceOrTemplate: props.mode,
			});
		} catch (e) { }
	});

	let [from, setFrom] = createSignal<string[] | undefined>();

	let [config, configOperations] = createResource(async () => {
		if (props.creating) {
			return undefined;
		}
		// Get the instance or template
		try {
			let configuration = await readEditableInstanceConfig(id, props.mode);
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
			return await getParentTemplates(from(), props.mode);
		},
		{ initialValue: [] }
	);

	let [releaseVersionsOnly, setReleaseVersionsOnly] = createSignal(true);

	let [supportedMinecraftVersions, supportedVersionsMethods] = createResource(async () => {
		let availableVersions = (await invoke("get_minecraft_versions", {
			releasesOnly: releaseVersionsOnly(),
		})) as string[];

		availableVersions.reverse();
		if (releaseVersionsOnly()) {
			return ["latest"].concat(availableVersions);
		} else {
			return ["latest", "latest_snapshot"].concat(availableVersions);
		}
	});

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
	let [isIdDirty, setIsIdDirty] = createSignal(!props.creating);
	let [isTypeDirty, setIsTypeDirty] = createSignal(!props.creating);
	let [isIconDirty, setIsIconDirty] = createSignal(!props.creating);
	let [isVersionDirty, setIsVersionDirty] = createSignal(!props.creating);
	let [isLoaderDirty, setIsLoaderDirty] = createSignal(!props.creating);

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

	let [displayName, setDisplayName] = createSignal("");
	let message = () =>
		isInstance ? `INSTANCE` : isBaseTemplate ? "BASE TEMPLATE" : `TEMPLATE`;

	let derivedPackages = createMemo(() => {
		return getDerivedPackages(parentConfigs());
	});

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

			setDisplayName(config()!.name == undefined ? id : config()!.name!);
		}

		// Default side
		if (props.creating && props.mode == "instance") {
			setSide("client");
		}
	});

	// Writes configuration to disk
	async function saveConfig() {
		let configId = props.creating ? newId() : id;

		if (!isBaseTemplate && configId == undefined) {
			inputError("id");
			return;
		} else {
			clearInputError("id");
		}
		if (props.creating) {
			if (await idExists(configId!, props.mode)) {
				inputError("id");
				return;
			} else {
				clearInputError("id");
			}
		}

		if (isInstance && side() == undefined) {
			inputError("side");
			return;
		} else {
			clearInputError("side");
		}

		let finalVersion =
			version() == undefined
				? getDerivedValue(parentConfigs(), (x) => x.version)
				: version();
		if (isInstance && finalVersion == undefined) {
			inputError("version");
			return;
		} else {
			clearInputError("version");
		}

		// The user selected custom Java but didn't pick anything
		if (javaType() == "" || javaType() == "custom") {
			inputError("launch-custom-java");
			inputError("launch-tab");
			return;
		} else {
			clearInputError("launch-custom-java");
			clearInputError("launch-tab");
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
				if (isInstance) {
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
			isInstance
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
			loader: loader() as Loader | undefined,
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
			await saveInstanceConfig(configId, newConfig, props.mode);

			successToast("Changes saved");

			props.setFooterData({
				selectedItem: undefined,
				mode: isInstance
					? FooterMode.SaveInstanceConfig
					: FooterMode.SaveTemplateConfig,
				action: saveConfig,
			});

			updateInstanceList();

			if (props.creating) {
				navigate("/");
			}

			configOperations.refetch();
			parentConfigOperations.refetch();
		} catch (e) {
			errorToast(e as string);
		}
	}

	let createMessage = isInstance ? "INSTANCE" : "TEMPLATE";

	// Highlights the save button when config changes
	function setDirty() {
		props.setFooterData({
			selectedItem: "",
			mode: isInstance
				? FooterMode.SaveInstanceConfig
				: FooterMode.SaveTemplateConfig,
			action: saveConfig,
		});
	}

	return (
		<div class="cont col" style="width:100%">
			<h2 id="head" class="noselect">
				{props.creating
					? `CREATING NEW ${createMessage}`
					: `CONFIGURE ${message()}`}
			</h2>
			<Show when={!props.creating}>
				<h3 id="subheader" class="noselect">
					{displayName()}
				</h3>
			</Show>
			<div class="cont">
				<div class="shadow" id="config-tabs">
					<Tip tip="General settings" side="top">
						<div
							class={`config-tab ${tab() == "general" ? "selected" : ""}`}
							id="general-tab"
							onclick={() => {
								setTab("general");
							}}
						>
							General
						</div>
					</Tip>
					<div
						class={`config-tab ${tab() == "packages" ? "selected" : ""}`}
						id="packages-tab"
						onclick={() => {
							setTab("packages");
						}}
					>
						Packages
					</div>
					<div
						class={`config-tab ${tab() == "launch" ? "selected" : ""}`}
						id="launch-tab"
						onclick={() => {
							setTab("launch");
						}}
					>
						Launch Settings
					</div>
				</div>
			</div>
			<br />
			<DisplayShow when={tab() == "general"}>
				<div class="fields">
					{/* <h3>Basic Settings</h3> */}
					<Show when={!isBaseTemplate}>
						<div class="cont start label">
							<label for="from">INHERIT CONFIG</label>
						</div>
						<Tip
							tip="A list of templates to inherit configuration from"
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
									setDirty();
								}}
								isSearchable={false}
								zIndex="50"
							/>
						</Tip>
					</Show>
					<Show when={!isBaseTemplate}>
						<label for="name" class="label">
							DISPLAY NAME
						</label>
						<Tip
							tip={`The name of the ${beautifyString(props.mode)}`}
							fullwidth
						>
							<input
								type="text"
								id="name"
								name="name"
								placeholder={id}
								value={emptyUndefined(name())}
								onChange={(e) => {
									setName(e.target.value);
									setDirty();
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
											if (loader != undefined && lowercaseName.includes(loader)) {
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
											setIcon(getLoaderImage(autofillLoader as Loader));
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
					<Show when={props.creating && !isBaseTemplate}>
						<label for="id" class="label">{`${createMessage} ID`}</label>
						<Tip tip="A unique name used to identify the instance" fullwidth>
							<input
								type="text"
								id="id"
								name="id"
								onChange={(e) => {
									setNewId();
									e.target.value = sanitizeInstanceId(e.target.value);
									setNewId(e.target.value);
									setDirty();
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
					<Show when={!isBaseTemplate}>
						<div class="cont start label">
							<label for="side">
								ICON
							</label>
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
								setDirty();
								setIsIconDirty(true);
							}}
							derivedIcon={getDerivedValue(parentConfigs(), (x) => x.icon)}
						/>
					</Show>
					<hr />
					<Show when={props.creating || isTemplate || isBaseTemplate}>
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
							fullwidth
						>
							<InlineSelect
								onChange={(x) => {
									setSide(x as "client" | "server" | undefined);
									setDirty();
									setIsTypeDirty(true);
								}}
								selected={side()}
								options={[
									{
										value: "client",
										contents: <div class="cont"><Icon icon={Controller} size="1.2rem" /> Client</div>,
										color: "var(--instance)",
									},
									{
										value: "server",
										contents: <div class="cont"><Icon icon={Server} size="1rem" /> Server</div>,
										color: "var(--template)",
									},
								]}
								columns={isInstance ? 2 : 3}
								allowEmpty={!isInstance}
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
						<Tip tip="The Minecraft version of this instance" fullwidth>
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
											setDirty();
											setIsVersionDirty(true);
										}}
										allowEmpty
										zIndex="50"
									/>
								</div>
								<div class="cont">
									<SlideSwitch
										enabled={!releaseVersionsOnly()}
										onToggle={() => setReleaseVersionsOnly(!releaseVersionsOnly())}
										enabledColor="var(--instance)"
										disabledColor="var(--fg3)"
									/>
									<span class="bold" style={`color:${releaseVersionsOnly() ? "var(--fg3)" : "var(--instance)"}`}>Include Snapshots</span>
								</div>
							</div>
						</Tip>
					</Show>
					<Show
						when={
							(side() == "client" || isTemplate) &&
							supportedLoaders() != undefined
						}
					>
						<div class="cont start label">
							<label for="client-type">{`${isTemplate ? "CLIENT " : ""
								}LOADER`}</label>
							<DeriveIndicator
								parentConfigs={parentConfigs()}
								currentValue={clientLoader()}
								property={(x) => {
									let loader = getConfiguredLoader(x.loader, "client");
									return loader == undefined
										? undefined
										: getLoaderDisplayName(loader);
								}}
							/>
						</div>
						<Tip
							tip={
								isInstance
									? "The loader to use. Install more with plugins!"
									: "The loader to use for client instances. Install more with plugins!"
							}
							fullwidth
						>
							<InlineSelect
								onChange={(x) => {
									setClientLoader(x);
									setDirty();
									setIsLoaderDirty(true);
								}}
								selected={clientLoader()}
								options={supportedLoaders()!
									.filter((x) => getLoaderSide(x) != "server")
									.map((x) => {
										return {
											value: x,
											contents: (
												<div
													class={`cont ${clientLoader() == undefined &&
														getDerivedValue(parentConfigs(), (x) =>
															getConfiguredLoader(x.loader, "client")
														) == x
														? "derived-option"
														: ""
														}`}
												>
													<Show when={x != undefined}>
														<img src={getLoaderImage(x as Loader)} style="width:1.2rem" />
													</Show>
													{x == undefined
														? "Unset"
														: getLoaderDisplayName(x as Loader)}
												</div>
											),
											color: getLoaderColor(x as Loader),
											tip:
												x == undefined ? "Inherit from the template" : undefined,
										};
									})}
								columns={3}
								allowEmpty={false}
								connected={false}
							/>
						</Tip>
					</Show>
					<Show
						when={
							(side() == "server" || isTemplate) &&
							supportedLoaders() != undefined
						}
					>
						<div class="cont start label">
							<label for="server-type">{`${isTemplate ? "SERVER " : ""
								}LOADER`}</label>
							<DeriveIndicator
								parentConfigs={parentConfigs()}
								currentValue={clientLoader()}
								property={(x) => {
									let loader = getConfiguredLoader(x.loader, "server");
									return loader == undefined
										? undefined
										: getLoaderDisplayName(loader);
								}}
							/>
						</div>
						<Tip
							tip={
								isInstance
									? "The loader to use. Install more with plugins!"
									: "The loader to use for server instances. Install more with plugins!"
							}
							fullwidth
						>
							<InlineSelect
								onChange={(x) => {
									setServerLoader(x);
									setDirty();
									setIsLoaderDirty(true);
								}}
								selected={serverLoader()}
								options={supportedLoaders()!
									.filter((x) => getLoaderSide(x) != "client")
									.map((x) => {
										return {
											value: x,
											contents: (
												<div
													class={`cont ${serverLoader() == undefined &&
														getDerivedValue(parentConfigs(), (x) =>
															getConfiguredLoader(x.loader, "server")
														) == x
														? "derived-option"
														: ""
														}`}
												>
													<Show when={x != undefined}>
														<img src={getLoaderImage(x as Loader)} style="width:1.2rem" />
													</Show>
													{x == undefined
														? "Unset"
														: getLoaderDisplayName(x as Loader)}
												</div>
											),
											color: getLoaderColor(x as Loader),
											tip:
												x == undefined ? "Inherit from the template" : undefined,
										};
									})}
								columns={3}
								allowEmpty={false}
								connected={false}
							/>
						</Tip>
					</Show>
					<Show
						when={
							side() == "client" &&
							(clientLoader() != undefined ||
								parentConfigs().some(
									(x) => getConfiguredLoader(x.loader, "client") != undefined
								))
						}
					>
						<div class="cont start label">
							<label for="client-loader-version">
								{isTemplate ? "CLIENT LOADER VERSION" : "LOADER VERSION"}
							</label>
						</div>
						<Tip
							tip={`The version for the${isTemplate ? " client" : ""
								} loader. Leave empty to select the best version automatically.`}
							fullwidth
						>
							<input
								type="text"
								id="client-loader-version"
								name="client-loader-version"
								value={emptyUndefined(clientLoaderVersion())}
								onChange={(e) => {
									setClientLoaderVersion(e.target.value);
									setDirty();
								}}
							></input>
						</Tip>
					</Show>
					<Show
						when={
							side() == "server" &&
							(serverLoader() != undefined ||
								parentConfigs().some(
									(x) => getConfiguredLoader(x.loader, "server") != undefined
								))
						}
					>
						<div class="cont start label">
							<label for="server-loader-version">
								{isTemplate ? "SERVER LOADER VERSION" : "LOADER VERSION"}
							</label>
						</div>
						<Tip
							tip={`The version for the${isTemplate ? " server" : ""
								} loader. Leave empty to select the best version automatically.`}
							fullwidth
						>
							<input
								type="text"
								id="server-loader-version"
								name="server-loader-version"
								value={emptyUndefined(serverLoaderVersion())}
								onChange={(e) => {
									setServerLoaderVersion(e.target.value);
									setDirty();
								}}
							></input>
						</Tip>
					</Show>
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
								setDirty();
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
			<DisplayShow when={tab() == "packages"}>
				<PackagesConfig
					id={id}
					isTemplate={isTemplate}
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

						setDirty();
					}}
					onAdd={(pkg, category) => {
						let func = (packages: PackageConfig[]) => {
							if (!packages.some((x) => packageConfigsEqual(x, pkg))) {
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

						setDirty();
					}}
					setGlobalPackages={(packages) => {
						setGlobalPackages(packages);
						setDirty();
					}}
					setClientPackages={(packages) => {
						setClientPackages(packages);
						setDirty();
					}}
					setServerPackages={(packages) => {
						setServerPackages(packages);
						setDirty();
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
					showBrowseButton={true}
					parentConfigs={parentConfigs()}
					onChange={setDirty}
					overrides={packageOverrides()}
					setOverrides={setPackageOverrides}
					beforeUpdate={saveConfig}
				/>
			</DisplayShow>
			<DisplayShow when={tab() == "launch"}>
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
					onChange={setDirty}
				/>
			</DisplayShow>
			<br />
			<br />
			<br />
		</div>
	);
}

export interface InstanceConfigProps {
	mode: InstanceConfigMode;
	/* Whether we are creating a new instance or template */
	creating: boolean;
	setFooterData: (data: FooterData) => void;
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
