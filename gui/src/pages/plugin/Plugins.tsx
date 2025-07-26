import { invoke } from "@tauri-apps/api";
import { createResource, createSignal, For, Show } from "solid-js";
import "./Plugins.css";
import IconTextButton from "../../components/input/IconTextButton";
import {
	Book,
	Box,
	CurlyBraces,
	Cycle,
	Download,
	Folder,
	Gear,
	Globe,
	Graph,
	Language,
	Link,
	Refresh,
	Text,
} from "../../icons";
import { emit } from "@tauri-apps/api/event";
import { errorToast, successToast } from "../../components/dialog/Toasts";
import Icon from "../../components/Icon";

export default function Plugins() {
	let [localPlugins, localMethods] = createResource(
		async () => (await invoke("get_local_plugins")) as PluginInfo[]
	);
	let [remotePlugins, remoteMethods] = createResource(
		async () => (await invoke("get_remote_plugins")) as PluginInfo[]
	);
	let [isRemote, setIsRemote] = createSignal(false);
	let [restartNeeded, setRestartNeeded] = createSignal(false);

	return (
		<div id="plugins">
			<div id="plugins-header">
				<div class="cont">
					<IconTextButton
						icon={Refresh}
						text="Refresh Launcher"
						size="22px"
						color="var(--bg2)"
						selectedColor="var(--plugin)"
						onClick={() => {
							emit("refresh_window");
						}}
						selected={restartNeeded()}
					/>
				</div>
				<h1 class="noselect">Plugins</h1>
				<div></div>
			</div>
			<div class="cont">
				<div id="plugins-subheader">
					<div
						class={`plugins-header-item ${isRemote() ? "" : " selected"}`}
						onclick={() => {
							setIsRemote(false);
						}}
					>
						Installed
					</div>
					<div
						class={`plugins-header-item ${isRemote() ? " selected" : ""}`}
						onclick={() => {
							setIsRemote(true);
						}}
					>
						Available
					</div>
				</div>
			</div>
			<br />
			<div class="cont col" id="plugin-list">
				<For each={localPlugins()}>
					{(info) => {
						return (
							<Show when={!isRemote()}>
								<Plugin
									info={info}
									updatePluginList={() => {
										localMethods.refetch();
										remoteMethods.refetch();
										setRestartNeeded(true);
									}}
								/>
							</Show>
						);
					}}
				</For>
				<Show when={localPlugins() != undefined}>
					<For each={remotePlugins()}>
						{(info) => {
							// Hide the remote version of a plugin if it is installed locally
							let idCount = () =>
								localPlugins()!.filter((x) => x.id == info.id).length;
							let isRemoteHidden = () => idCount() >= 1;
							return (
								<Show when={isRemote() && !isRemoteHidden()}>
									<Plugin
										info={info}
										updatePluginList={() => {
											localMethods.refetch();
											remoteMethods.refetch();
											setRestartNeeded(true);
										}}
									/>
								</Show>
							);
						}}
					</For>
				</Show>
			</div>
			<br />
			<br />
			<br />
			<br />
		</div>
	);
}

function Plugin(props: PluginProps) {
	let isDisabled = () => !props.info.enabled && props.info.installed;

	let [inProgress, setInProgress] = createSignal(false);

	return (
		<div
			class={`cont col input-shadow plugin ${isDisabled() ? "disabled" : ""}`}
		>
			<div class="plugin-top">
				<div class="cont plugin-header">
					<div class="cont plugin-icon">{getPluginIcon(props.info.id)}</div>
					<div class="plugin-name">{props.info.name}</div>
					<div class="plugin-id">{props.info.id}</div>
				</div>
				<div class="cont plugin-buttons">
					<Show when={props.info.installed}>
						<IconTextButton
							text={props.info.enabled ? "Disable" : "Enable"}
							size="22px"
							color="var(--bg2)"
							selectedColor="var(--instance)"
							onClick={() => {
								invoke("enable_disable_plugin", {
									plugin: props.info.id,
									enabled: !props.info.enabled,
								}).then(() => {
									successToast(
										`Plugin ${props.info.enabled ? "disabled" : "enabled"}`
									);
									props.updatePluginList();
								});
							}}
							selected={false}
							shadow={false}
						/>
						<IconTextButton
							text="Update"
							size="22px"
							color="var(--bg2)"
							selectedColor="var(--instance)"
							onClick={() => {
								setInProgress(true);
								invoke("install_plugin", {
									plugin: props.info.id,
								}).then(
									() => {
										setInProgress(false);
										successToast("Plugin updated");
										props.updatePluginList();
									},
									(e) => {
										setInProgress(false);
										errorToast(`Failed to update plugin: ${e}`);
									}
								);
							}}
							selected={false}
							shadow={false}
						/>
					</Show>
					<IconTextButton
						text={
							props.info.installed
								? "Uninstall"
								: inProgress()
								? "Installing..."
								: "Install"
						}
						size="22px"
						color="var(--bg2)"
						selectedColor="var(--instance)"
						onClick={() => {
							setInProgress(true);
							let method = props.info.installed
								? "uninstall_plugin"
								: "install_plugin";
							invoke(method, {
								plugin: props.info.id,
							}).then(
								() => {
									setInProgress(false);
									successToast(
										`Plugin ${
											props.info.installed ? "uninstalled" : "installed"
										}`
									);
									props.updatePluginList();
								},
								(e) => {
									setInProgress(false);
									errorToast(
										`Failed to ${
											props.info.installed ? "uninstall" : "install"
										} plugin: ${e}`
									);
								}
							);
						}}
						selected={false}
						shadow={false}
					/>
				</div>
			</div>
			<div class="cont" style="justify-content:flex-start;width:100%">
				<div class="plugin-description">{props.info.description}</div>
			</div>
		</div>
	);
}

interface PluginProps {
	info: PluginInfo;
	updatePluginList: () => void;
}

interface PluginInfo {
	id: string;
	name?: string;
	description?: string;
	enabled: boolean;
	installed: boolean;
}

function getPluginIcon(plugin: string) {
	let imageIcon = (() => {
		if (plugin == "fabric_quilt") {
			return "/icons/fabric.png";
		} else if (plugin == "paper") {
			return "/icons/paper.png";
		} else if (plugin == "sponge") {
			return "/icons/sponge.png";
		}
	})();

	if (imageIcon != undefined) {
		return <img src={imageIcon} style="width:1rem" />;
	}

	let svgIcon = (() => {
		if (plugin == "args") {
			return Text;
		} else if (plugin == "automate") {
			return Gear;
		} else if (plugin == "backup") {
			return Download;
		} else if (plugin == "config_split") {
			return Gear;
		} else if (plugin == "custom_files") {
			return Folder;
		} else if (plugin == "docs") {
			return Book;
		} else if (plugin == "extra_versions") {
			return CurlyBraces;
		} else if (plugin == "lang") {
			return Language;
		} else if (plugin == "nitro_transfer") {
			return Cycle;
		} else if (plugin == "options") {
			return Gear;
		} else if (plugin == "server_restart") {
			return Refresh;
		} else if (plugin == "stats") {
			return Graph;
		} else if (plugin == "webtools") {
			return Globe;
		} else if (plugin == "weld") {
			return Link;
		} else if (plugin == "xmcl_transfer") {
			return Cycle;
		}

		return Box;
	})();

	return <Icon icon={svgIcon} size="1rem" />;
}
