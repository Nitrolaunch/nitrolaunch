import { useParams } from "@solidjs/router";
import "./InstanceConfig.css";
import { invoke } from "@tauri-apps/api";
import {
	createEffect,
	createResource,
	createSignal,
	onMount,
	Show,
} from "solid-js";
import "@thisbeyond/solid-select/style.css";
import InlineSelect from "../../components/input/InlineSelect";
import { loadPagePlugins } from "../../plugins";
import { inputError } from "../../errors";
import {
	beautifyString,
	getSupportedLoaders,
	parseVersionedString,
} from "../../utils";
import { FooterData } from "../../App";
import { FooterMode } from "../../components/navigation/Footer";
import PackagesConfig, {
	getPackageConfigRequest,
	PackageConfig,
} from "./PackagesConfig";
import Tip from "../../components/dialog/Tip";
import { errorToast, successToast } from "../../components/dialog/Toasts";
import DisplayShow from "../../components/utility/DisplayShow";
import {
	getLoaderColor,
	getLoaderDisplayName,
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
	getDerivedValue,
	InstanceConfigMode,
	readEditableInstanceConfig,
	saveInstanceConfig,
} from "./read_write";
import { InstanceConfig } from "./read_write";
import DeriveIndicator from "./DeriveIndicator";
import { InstanceInfo } from "../../types";
import Dropdown from "../../components/input/Dropdown";
import LoadingSpinner from "../../components/utility/LoadingSpinner";

export default function InstanceConfigPage(props: InstanceConfigProps) {
	let params = useParams();

	let isInstance = props.mode == InstanceConfigMode.Instance;
	let isProfile = props.mode == InstanceConfigMode.Profile;
	let isGlobalProfile = props.mode == InstanceConfigMode.GlobalProfile;

	let id = isInstance
		? params.instanceId
		: isGlobalProfile
		? "Global Profile"
		: params.profileId;

	onMount(() =>
		loadPagePlugins(
			isInstance
				? "instance_config"
				: isProfile
				? "profile_config"
				: "global_profile_config",
			id
		)
	);

	createEffect(() => {
		props.setFooterData({
			selectedItem: props.creating ? "" : undefined,
			mode: isInstance
				? FooterMode.SaveInstanceConfig
				: FooterMode.SaveProfileConfig,
			action: saveConfig,
		});
	});

	let [from, setFrom] = createSignal<string[] | undefined>();

	let [config, configOperations] = createResource(async () => {
		if (props.creating) {
			return undefined;
		}
		// Get the instance or profile
		try {
			let configuration = await readEditableInstanceConfig(id, props.mode);
			setFrom(canonicalizeListOrSingle(configuration.from));
			return configuration;
		} catch (e) {
			errorToast("Failed to load configuration: " + e);
			return undefined;
		}
	});
	let [parentConfigs, parentConfigOperations] = createResource(
		() => from(),
		async () => {
			let fromValues = from();
			// Get the parent
			let parentResults: InstanceConfig[] = [];
			if (isGlobalProfile) {
				parentResults = [];
			} else if (fromValues == undefined) {
				let parentResult = await invoke("get_global_profile", {});
				parentResults = [parentResult as InstanceConfig];
			} else {
				for (let profile of fromValues!) {
					let parentResult = await invoke("get_profile_config", {
						id: profile,
					});
					parentResults.push(parentResult as InstanceConfig);
				}
			}

			return parentResults;
		},
		{ initialValue: [] }
	);

	let [supportedMinecraftVersions, _] = createResource(async () => {
		let availableVersions = (await invoke("get_minecraft_versions", {
			releasesOnly: false,
		})) as string[];

		availableVersions.reverse();
		return ["latest", "latest_snapshot"].concat(availableVersions);
	});

	let [supportedLoaders, __] = createResource(async () => {
		let loaders = await getSupportedLoaders();
		return [undefined as string | undefined].concat(loaders);
	});

	// Available profiles to derive from
	let [profiles, ___] = createResource(async () => {
		return (await invoke("get_profiles")) as InstanceInfo[];
	});

	let [tab, setTab] = createSignal("general");

	// Input / convenience signals

	// Used to check if we can automatically fill out the ID with the name. We don't want to do this if the user already typed an ID.
	let [isIdDirty, setIsIdDirty] = createSignal(!props.creating);

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

	let [displayName, setDisplayName] = createSignal("");
	let message = () =>
		isInstance ? `INSTANCE` : isGlobalProfile ? "GLOBAL PROFILE" : `PROFILE`;

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

			let [global, client, server] = getConfigPackages(config()!);
			setGlobalPackages(global);
			setClientPackages(client);
			setServerPackages(server);

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

		if (!isGlobalProfile && configId == undefined) {
			inputError("id");
			return;
		}
		if (props.creating) {
			if (await idExists(configId!, props.mode)) {
				inputError("id");
				return;
			}
		}

		if (isInstance && side() == undefined) {
			inputError("side");
			return;
		}

		if (isInstance && version() == undefined) {
			inputError("version");
			return;
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

		let newConfig: InstanceConfig = {
			from: from(),
			type: side(),
			name: undefinedEmpty(name()),
			icon: undefinedEmpty(icon()),
			version: undefinedEmpty(version()),
			loader: loader() as Loader | undefined,
			packages: packages,
		};

		// Handle extra fields
		if (config() != undefined) {
			for (let key of Object.keys(config()!)) {
				if (!Object.keys(newConfig).includes(key)) {
					newConfig[key] = config()![key];
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
					: FooterMode.SaveProfileConfig,
				action: saveConfig,
			});

			configOperations.refetch();
			parentConfigOperations.refetch();
		} catch (e) {
			errorToast(e as string);
		}
	}

	let createMessage = isInstance ? "INSTANCE" : "PROFILE";

	// Highlights the save button when config changes
	function setDirty() {
		props.setFooterData({
			selectedItem: "",
			mode: isInstance
				? FooterMode.SaveInstanceConfig
				: FooterMode.SaveProfileConfig,
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
				<div class="input-shadow" id="config-tabs">
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
					<Show when={!isGlobalProfile}>
						<div class="cont start label">
							<label for="from">INHERIT CONFIG</label>
						</div>
						<Tip
							tip="A list of profiles to inherit configuration from"
							fullwidth
						>
							<Dropdown
								options={
									profiles() == undefined
										? []
										: profiles()!.map((x) => {
												return {
													value: x.id,
													contents: (
														<div>{x.name == undefined ? x.id : x.name}</div>
													),
													color: "var(--profile)",
												};
										  })
								}
								selected={from()}
								onChangeMulti={(x) => {
									setFrom(x);
									setDirty();
								}}
								zIndex="50"
							/>
						</Tip>
					</Show>
					<Show when={!isGlobalProfile && !isProfile}>
						<label for="name" class="label">
							DISPLAY NAME
						</label>
						<Tip tip="The name of the instance" fullwidth>
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
									if (!isIdDirty()) {
										let value = sanitizeInstanceId(e.target.value);
										(document.getElementById("id")! as any).value = value;
										setNewId(value);
									}
								}}
							></input>
						</Tip>
					</Show>
					<Show when={props.creating && !isGlobalProfile}>
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
									setIsIdDirty(true);
									e.target.value = sanitizeInstanceId(e.target.value);
								}}
							></input>
						</Tip>
					</Show>
					<Show when={props.creating || isProfile || isGlobalProfile}>
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
								}}
								selected={side()}
								options={[
									{
										value: "client",
										contents: <div class="cont">Client</div>,
										color: "var(--instance)",
									},
									{
										value: "server",
										contents: <div class="cont">Server</div>,
										color: "var(--profile)",
									},
								]}
								columns={isInstance ? 2 : 3}
								allowEmpty={!isInstance}
							/>
						</Tip>
					</Show>
					<hr />
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
								}}
								allowEmpty
								zIndex="50"
							/>
						</Tip>
					</Show>
					<Show
						when={
							(side() == "client" || isProfile) &&
							supportedLoaders() != undefined
						}
					>
						<div class="cont start label">
							<label for="client-type">{`${
								isProfile ? "CLIENT " : ""
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
									? "The loader to use"
									: "The loader to use for client instances"
							}
							fullwidth
						>
							<InlineSelect
								onChange={(x) => {
									setClientLoader(x);
									setDirty();
								}}
								selected={clientLoader()}
								options={supportedLoaders()!
									.filter((x) => getLoaderSide(x) != "server")
									.map((x) => {
										return {
											value: x,
											contents: (
												<div
													class={`cont ${
														clientLoader() == undefined &&
														getDerivedValue(parentConfigs(), (x) =>
															getConfiguredLoader(x.loader, "client")
														) == x
															? "derived-option"
															: ""
													}`}
												>
													{x == undefined
														? "Unset"
														: getLoaderDisplayName(x as Loader)}
												</div>
											),
											color: getLoaderColor(x as Loader),
											tip:
												x == undefined ? "Inherit from the profile" : undefined,
										};
									})}
								columns={4}
								allowEmpty={false}
								connected={false}
							/>
						</Tip>
					</Show>
					<Show
						when={
							(side() == "server" || isProfile) &&
							supportedLoaders() != undefined
						}
					>
						<div class="cont start label">
							<label for="server-type">{`${
								isProfile ? "SERVER " : ""
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
									? "The loader to use"
									: "The loader to use for server instances"
							}
							fullwidth
						>
							<InlineSelect
								onChange={(x) => {
									setServerLoader(x);
									setDirty();
								}}
								selected={serverLoader()}
								options={supportedLoaders()!
									.filter((x) => getLoaderSide(x) != "client")
									.map((x) => {
										return {
											value: x,
											contents: (
												<div
													class={`cont ${
														serverLoader() == undefined &&
														getDerivedValue(parentConfigs(), (x) =>
															getConfiguredLoader(x.loader, "server")
														) == x
															? "derived-option"
															: ""
													}`}
												>
													{x == undefined
														? "Unset"
														: getLoaderDisplayName(x as Loader)}
												</div>
											),
											color: getLoaderColor(x as Loader),
											tip:
												x == undefined ? "Inherit from the profile" : undefined,
										};
									})}
								columns={4}
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
								{isProfile ? "CLIENT LOADER VERSION" : "LOADER VERSION"}
							</label>
						</div>
						<Tip
							tip={`The version for the${
								isProfile ? " client" : ""
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
								{isProfile ? "SERVER LOADER VERSION" : "LOADER VERSION"}
							</label>
						</div>
						<Tip
							tip={`The version for the${
								isProfile ? " server" : ""
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
							class="profile-placeholder"
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
					isProfile={isProfile}
					globalPackages={globalPackages()}
					clientPackages={clientPackages()}
					serverPackages={serverPackages()}
					onRemove={(pkg, category) => {
						if (category == "global") {
							setGlobalPackages((packages) =>
								packages.filter((x) => getPackageConfigRequest(x).id != pkg)
							);
						} else if (category == "client") {
							setClientPackages((packages) =>
								packages.filter((x) => getPackageConfigRequest(x).id != pkg)
							);
						} else if (category == "server") {
							setServerPackages((packages) =>
								packages.filter((x) => getPackageConfigRequest(x).id != pkg)
							);
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
	/* Whether we are creating a new instance or profile */
	creating: boolean;
	setFooterData: (data: FooterData) => void;
}

// Sanitizes a string so that it is a valid instance ID
function sanitizeInstanceId(id: string): string {
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

// Checks if an instance or profile ID exists already
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
