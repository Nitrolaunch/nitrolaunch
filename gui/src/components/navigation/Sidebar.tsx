import {
	createResource,
	createSignal,
	For,
	JSX,
	onCleanup,
	Show,
} from "solid-js";
import "./Sidebar.css";
import { Box, Gear, Menu } from "../../icons";
import { Location, useNavigate } from "@solidjs/router";
import { invoke } from "@tauri-apps/api/core";
import { getInstanceIconSrc, stringCompare } from "../../utils";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { InstanceInfo, InstanceOrTemplate } from "../../types";
import Icon from "../Icon";
import { setInstanceConfigModal } from "../../App";
import { InstanceConfigMode } from "../../pages/instance/read_write";
import Settings from "../../pages/Settings";
import { Portal } from "solid-js/web";

export default function Sidebar(props: SidebarProps) {
	let [extraButtons, _] = createResource(async () => {
		try {
			let buttons: PluginSidebarButton[] = await invoke("get_sidebar_buttons");
			buttons.sort((a, b) => stringCompare(a.href, b.href));
			return buttons;
		} catch (e) {
			console.error("Failed to load sidebar buttons: " + e);
			return undefined;
		}
	});

	let [unlisten, setUnlisten] = createSignal<UnlistenFn | undefined>();
	let [instanceButtons, instanceButtonMethods] = createResource(async () => {
		// Listener for when the last opened instance updates
		let unlisten = await listen("nitro_update_last_opened_instance", () => {
			instanceButtonMethods.refetch();
		});
		setUnlisten(() => unlisten);

		let [instances, templates, lastOpenedInstance] = (await Promise.all([
			invoke("get_instances"),
			invoke("get_templates"),
			invoke("get_last_opened_instance"),
		])) as [
			InstanceInfo[],
			InstanceInfo[],
			[string, InstanceOrTemplate] | undefined,
		];

		let allInstances: [InstanceInfo, InstanceOrTemplate][] = [];

		if (lastOpenedInstance != undefined) {
			let source = lastOpenedInstance[1] == "instance" ? instances : templates;
			let info = source.find((x) => x.id == lastOpenedInstance[0]);
			if (info != undefined) {
				allInstances.push([info, lastOpenedInstance[1]]);
			}
		}

		for (let info of instances) {
			if (info.pinned) {
				if (
					lastOpenedInstance != undefined &&
					lastOpenedInstance[0] == info.id &&
					lastOpenedInstance[1] == "instance"
				) {
					continue;
				}

				allInstances.push([info, "instance"]);
			}
		}

		for (let info of templates) {
			if (info.pinned) {
				if (
					lastOpenedInstance != undefined &&
					lastOpenedInstance[0] == info.id &&
					lastOpenedInstance[1] == "template"
				) {
					continue;
				}

				allInstances.push([info, "template"]);
			}
		}

		return allInstances;
	});

	onCleanup(() => {
		if (unlisten() != undefined) {
			unlisten()!();
		}
	});

	let [settingsVisible, setSettingsVisible] = createSignal(false);

	let [nitroVersion, __] = createResource(async () => {
		return (await invoke("get_nitro_version")) as string;
	});

	return (
		<div
			class="cont col start"
			id="sidebar"
			style={`${
				props.visible
					? ""
					: "width:0px;border-right-color:var(--bg);opacity:0%;pointer-events:none"
			}`}
			onmouseleave={() => props.setVisible(false)}
		>
			<div class="cont">
				<Show when={nitroVersion() != undefined}>
					<div class="cont bold" style="color:var(--fg3)">
						v{nitroVersion()}
					</div>
				</Show>
			</div>
			<div class="cont col" id="sidebar-items">
				<SidebarItem
					href="/docs"
					location={props.location}
					color="var(--instance)"
					selectedBg="var(--instancebg)"
					onClick={() => {
						setSettingsVisible(true);
					}}
					closeSidebar={() => props.setVisible(false)}
				>
					<div class="cont">
						<Icon icon={Gear} size="1rem" />
					</div>
					<div class="cont">Settings</div>
				</SidebarItem>
				<SidebarItem
					href="/docs"
					location={props.location}
					selectedPathStart="/docs"
					color="var(--template)"
					selectedBg="var(--templatebg)"
					closeSidebar={() => props.setVisible(false)}
				>
					<div class="cont">
						<Icon icon={Menu} size="1rem" />
					</div>
					<div class="cont">Documentation</div>
				</SidebarItem>
				<Show when={extraButtons() != undefined}>
					<For each={extraButtons()}>
						{(button) => (
							<SidebarItem
								innerhtml={button.html}
								href={button.href}
								location={props.location}
								selectedPath={button.selected_url}
								selectedPathStart={button.selected_url_start}
								color={button.color}
								closeSidebar={() => props.setVisible(false)}
							></SidebarItem>
						)}
					</For>
				</Show>
				<div class="cont sidebar-divider">INSTANCES</div>
				<For each={instanceButtons()}>
					{([info, type]) => {
						let url = type == "instance" ? `/instance/${info.id}` : "";

						let icon =
							info.icon == null ? (
								<Icon icon={Box} size="1.5rem" />
							) : (
								<img src={getInstanceIconSrc(info.icon)} style="width:1.5rem" />
							);

						return (
							<SidebarItem
								href={url}
								onClick={
									type == "instance"
										? undefined
										: () => {
												setInstanceConfigModal(
													info.id,
													InstanceConfigMode.Template,
													false,
												);
											}
								}
								location={props.location}
								selectedPath={url}
								color={`var(--${type})`}
								selectedBg={`var(--${type}bg)`}
								closeSidebar={() => props.setVisible(false)}
							>
								<div class="cont">{icon}</div>
								<div class="cont">
									{info.name == undefined ? info.id : info.name}
								</div>
							</SidebarItem>
						);
					}}
				</For>
			</div>
			<Portal>
				<Settings
					isVisible={settingsVisible()}
					onClose={() => setSettingsVisible(false)}
				/>
			</Portal>
		</div>
	);
}

export interface SidebarProps {
	visible: boolean;
	setVisible: (visible: boolean) => void;
	location: Location;
}

function SidebarItem(props: SidebarItemProps) {
	let navigate = useNavigate();

	const selected = () => {
		if (props.selectedPath != undefined) {
			return props.location.pathname == props.selectedPath;
		}
		if (props.selectedPathStart != undefined) {
			return props.location.pathname.startsWith(props.selectedPathStart);
		}

		return false;
	};

	let color = () => (selected() ? `color:${props.color}` : "");
	let bgColor = () =>
		!selected() || props.selectedBg == undefined
			? ""
			: `background-color:${props.selectedBg}`;

	return (
		<div
			class={`cont bubble-hover sidebar-item ${selected() ? "selected" : ""}`}
			style={`${selected() ? `border-color:${props.color};` : ""}${color()};${bgColor()}`}
			onclick={() => {
				if (props.onClick != undefined) {
					props.onClick();
				} else {
					navigate(props.href);
				}
				props.closeSidebar();
			}}
			innerHTML={props.innerhtml}
		>
			{props.children}
		</div>
	);
}

interface SidebarItemProps {
	children?: JSX.Element;
	innerhtml?: string;
	href: string;
	onClick?: () => void;
	location: Location;
	// What the current URL should equal to select this item
	selectedPath?: string;
	// What the current URL should start with to select this item
	selectedPathStart?: string;
	color: string;
	selectedBg?: string;
	closeSidebar: () => void;
}

interface PluginSidebarButton {
	html: string;
	href: string;
	selected_url?: string;
	selected_url_start?: string;
	color: string;
}
