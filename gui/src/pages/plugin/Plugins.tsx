import { invoke } from "@tauri-apps/api";
import { createResource, createSignal, For, Match, Show, Switch } from "solid-js";
import "./Plugins.css";
import IconTextButton from "../../components/input/IconTextButton";
import {
	Book,
	Box,
	Check,
	CurlyBraces,
	Cycle,
	Delete,
	Download,
	Folder,
	Gear,
	Globe,
	Graph,
	Language,
	Link,
	Refresh,
	Text,
	Trash,
} from "../../icons";
import { emit } from "@tauri-apps/api/event";
import { errorToast, successToast } from "../../components/dialog/Toasts";
import Icon from "../../components/Icon";
import Tip from "../../components/dialog/Tip";
import IconButton from "../../components/input/IconButton";

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
						class={`plugins-header-item bubble-hover ${isRemote() ? "" : " selected"}`}
						onclick={() => {
							setIsRemote(false);
						}}
					>
						Installed
					</div>
					<div
						class={`plugins-header-item bubble-hover ${isRemote() ? " selected" : ""}`}
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
						<Tip tip={props.info.enabled ? "Plugin Enabled" : "Plugin Disabled"} side="top">
							<Switch>
								<Match when={props.info.enabled}>
									<IconButton icon={Check} size="1.5rem" color="var(--pluginbg)" border="var(--plugin)" iconColor="var(--plugin)" onClick={() => {
										invoke("enable_disable_plugin", {
											plugin: props.info.id,
											enabled: !props.info.enabled,
										}).then(() => {
											successToast(
												`Plugin ${props.info.enabled ? "disabled" : "enabled"}`
											);
											props.updatePluginList();
										});
									}} />
								</Match>
								<Match when={!props.info.enabled}>
									<IconButton icon={Delete} size="1.5rem" color="var(--bg2)" border="var(--bg3)" iconColor="var(--fg3)" onClick={() => {
										invoke("enable_disable_plugin", {
											plugin: props.info.id,
											enabled: !props.info.enabled,
										}).then(() => {
											successToast(
												`Plugin ${props.info.enabled ? "disabled" : "enabled"}`
											);
											props.updatePluginList();
										});
									}} />
								</Match>
							</Switch>
						</Tip>
						<Tip tip="Update" side="top">
							<IconButton icon={Refresh} size="1.5rem" color="var(--bg2)" border="var(--bg3)" hoverBorder="var(--bg4)" hoverBackground="var(--bg3)" onClick={() => {
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
							}} /></Tip>
					</Show>
					<Tip tip={props.info.installed
						? "Uninstall"
						: inProgress()
							? "Installing..."
							: "Install"} side="top">
						<Switch>
							<Match when={props.info.installed}>
								<IconButton icon={Trash} size="1.5rem" color="var(--errorbg)" iconColor="var(--error)" border="var(--error)" onClick={() => {
									setInProgress(true);
									invoke("uninstall_plugin", {
										plugin: props.info.id,
									}).then(
										() => {
											setInProgress(false);
											successToast(
												`Plugin uninstalled`
											);
											props.updatePluginList();
										},
										(e) => {
											setInProgress(false);
											errorToast(
												`Failed to uninstall plugin: ${e}`
											);
										}
									);
								}} />
							</Match>
							<Match when={!props.info.installed}>
								<IconButton icon={Download} size="1.5rem" color="var(--bg2)" border="var(--bg3)" hoverBorder="var(--bg4)" hoverBackground="var(--bg3)" onClick={() => {
									setInProgress(true);
									invoke("install_plugin", {
										plugin: props.info.id,
									}).then(
										() => {
											setInProgress(false);
											successToast(
												`Plugin installed`
											);
											props.updatePluginList();
										},
										(e) => {
											setInProgress(false);
											errorToast(
												`Failed to install plugin: ${e}`
											);
										}
									);
								}} />
							</Match>
						</Switch>
					</Tip>
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
		if (plugin == "archive") {
			return Book;
		} else if (plugin == "args") {
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
		} else if (plugin == "extra_versions" || plugin == "better_jsons") {
			return CurlyBraces;
		} else if (plugin == "lang") {
			return Language;
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
		} else if (plugin.includes("transfer")) {
			return Cycle;
		}

		return Box;
	})();

	return <Icon icon={svgIcon} size="1rem" />;
}
