import { createResource, createSignal, Match, Show, Switch } from "solid-js";
import Tip from "../../components/dialog/Tip";
import { getLoaderColor, getLoaderDisplayName, getLoaderImage, getLoaderSide, Loader } from "../../package";
import { getConfiguredLoader, getDerivedValue, InstanceConfig } from "./read_write";
import { Side } from "../../types";
import { invoke } from "@tauri-apps/api/core";
import Dropdown from "../../components/input/select/Dropdown";
import DeriveIndicator from "./DeriveIndicator";
import InlineSelect from "../../components/input/select/InlineSelect";
import { errorToast } from "../../components/dialog/Toasts";

export default function LoaderConfig(props: LoaderConfigProps) {
	// Cache map of loader to supported versions
	let [supportedLoaderVersions, setSupportedLoaderVersions] = createSignal<{ [loader: string]: string[] }>({});
	let [clientLoaderVersions, setClientLoaderVersions] = createSignal<string[]>([]);
	let [serverLoaderVersions, setServerLoaderVersions] = createSignal<string[]>([]);

	let [supportedLoaderVersionsResource, _] = createResource(() => {
		// Weird stuff to make a proper source
		let client = props.clientLoader == undefined ? "" : props.clientLoader!;
		let server = props.serverLoader == undefined ? "" : props.serverLoader!;
		let vers = props.minecraftVersion == undefined ? "" : props.minecraftVersion;
		return client + server + vers;
	}, async () => {
		if (props.minecraftVersion == undefined) {
			return;
		}

		let loader = props.side == "client" ? props.clientLoader : props.side == "server" ? props.serverLoader : undefined;
		if (loader == undefined) {
			if (props.side == "client") {
				setClientLoaderVersions([]);
			} else if (props.side == "server") {
				setServerLoaderVersions([]);
			}
			return;
		}

		if (supportedLoaderVersions()[loader] == undefined) {
			try {
				let versions: string[] = await invoke(
					"get_loader_versions",
					{ loader: loader, minecraftVersion: props.minecraftVersion }
				);
				console.log(versions);

				setSupportedLoaderVersions((map) => {
					map[loader] = versions;
					return map;
				})
			} catch (e) {
				errorToast("Failed to get loader versions: " + e);
				return;
			}
		}
		let versions = supportedLoaderVersions()[loader];
		if (props.side == "client") {
			setClientLoaderVersions(versions);
		} else if (props.side == "server") {
			setServerLoaderVersions(versions);
		}
	});

	let showClientLoaderVersion = () => props.clientLoader != undefined ||
		props.parentConfigs.some((x) => getConfiguredLoader(x.loader, "client") != undefined);
	let showServerLoaderVersion = () => props.serverLoader != undefined ||
		props.parentConfigs.some((x) => getConfiguredLoader(x.loader, "server") != undefined);

	return <>
		<Show
			when={
				(props.side == "client" || props.isTemplate) &&
				props.supportedLoaders != undefined
			}
		>
			<div class="cont start label">
				<label for="client-type">{`${props.isTemplate ? "CLIENT " : ""
					}LOADER`}</label>
				<DeriveIndicator
					parentConfigs={props.parentConfigs}
					currentValue={props.clientLoader}
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
					props.isTemplate
						? "The loader to use for client instances. Install more with plugins!"
						: "The loader to use. Install more with plugins!"
				}
				fullwidth
			>
				<InlineSelect
					onChange={(x) => {
						props.setClientLoader(x as Loader | undefined);
						props.setDirty();
						props.setLoaderDirty();
					}}
					selected={props.clientLoader}
					options={props.supportedLoaders!
						.filter((x) => getLoaderSide(x) != "server")
						.map((x) => {
							return {
								value: x,
								contents: (
									<div
										class={`cont ${props.clientLoader == undefined &&
											getDerivedValue(props.parentConfigs, (x) =>
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
				(props.side == "server" || props.isTemplate) &&
				props.supportedLoaders != undefined
			}
		>
			<div class="cont start label">
				<label for="server-type">{`${props.isTemplate ? "SERVER " : ""
					}LOADER`}</label>
				<DeriveIndicator
					parentConfigs={props.parentConfigs}
					currentValue={props.clientLoader}
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
					props.isTemplate
						? "The loader to use for server instances. Install more with plugins!"
						: "The loader to use. Install more with plugins!"
				}
				fullwidth
			>
				<InlineSelect
					onChange={(x) => {
						props.setServerLoader(x as Loader | undefined);
						props.setDirty();
						props.setLoaderDirty();
					}}
					selected={props.serverLoader}
					options={props.supportedLoaders!
						.filter((x) => getLoaderSide(x) != "client")
						.map((x) => {
							return {
								value: x,
								contents: (
									<div
										class={`cont ${props.serverLoader == undefined &&
											getDerivedValue(props.parentConfigs, (x) =>
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
		<Switch>
			<Match when={props.minecraftVersion == undefined}>
				<Show
					when={
						showClientLoaderVersion() || showServerLoaderVersion()
					}
				>
					<div class="cont start label">
						LOADER VERSION
					</div>
					<div style="color:var(--fg3)">
						Select a Minecraft version first
					</div>
				</Show>
			</Match>
			<Match when={supportedLoaderVersionsResource.loading}>
				Fetching loader versions...
			</Match>
			<Match when={props.minecraftVersion != undefined}>
				<Show when={(props.side == "client" || props.isTemplate) && showClientLoaderVersion()}>
					<div class="cont start label">
						<label for="client-loader-version">
							{props.isTemplate ? "CLIENT LOADER VERSION" : "LOADER VERSION"}
						</label>
					</div>
					<Tip
						tip={`The version for the${props.isTemplate ? " client" : ""
							} loader. Leave empty to select the best version automatically.`}
						fullwidth
					>
						<Dropdown
							options={clientLoaderVersions()!.map((x) => {
								return {
									value: x,
									contents: x,
									color: "var(--package)",
								};
							})}
							selected={props.clientLoaderVersion}
							onChange={(x) => {
								props.setClientLoaderVersion(x);
								props.setDirty();
							}}
							allowEmpty
							zIndex="50"
						/>
					</Tip>
				</Show>
				<Show when={(props.side == "server" || props.isTemplate) && showServerLoaderVersion()}>
					<div class="cont start label">
						<label for="server-loader-version">
							{props.isTemplate ? "SERVER LOADER VERSION" : "LOADER VERSION"}
						</label>
					</div>
					<Tip
						tip={`The version for the${props.isTemplate ? " server" : ""
							} loader. Leave empty to select the best version automatically.`}
						fullwidth
					>
						<Dropdown
							options={serverLoaderVersions()!.map((x) => {
								return {
									value: x,
									contents: x,
									color: "var(--package)",
								};
							})}
							selected={props.serverLoaderVersion}
							onChange={(x) => {
								props.setServerLoaderVersion(x);
								props.setDirty();
							}}
							allowEmpty
							zIndex="50"
						/>
					</Tip>
				</Show>
			</Match>
		</Switch>
	</>;
}

export interface LoaderConfigProps {
	minecraftVersion?: string;
	side?: Side;
	isTemplate: boolean;

	clientLoader?: string;
	serverLoader?: string;
	clientLoaderVersion?: string;
	serverLoaderVersion?: string;

	setClientLoader: (loader: Loader | undefined) => void;
	setServerLoader: (loader: Loader | undefined) => void;
	setClientLoaderVersion: (version: string | undefined) => void;
	setServerLoaderVersion: (version: string | undefined) => void;

	supportedLoaders: (string | undefined)[] | undefined;

	parentConfigs: InstanceConfig[];
	setDirty: () => void;
	setLoaderDirty: () => void;
}
