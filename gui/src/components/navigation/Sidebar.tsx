import { createResource, For, JSX, onMount, Show } from "solid-js";
import "./Sidebar.css";
import { Box, Home, Jigsaw, Menu } from "../../icons";
import { Location } from "@solidjs/router";
import { invoke } from "@tauri-apps/api";
import { stringCompare } from "../../utils";

export default function Sidebar(props: SidebarProps) {
	// Close the sidebar when clicking outside of it
	onMount(() => {
		document.addEventListener("click", (e) => {
			let sidebar = document.getElementById("sidebar");
			let sidebarButton = document.getElementById("sidebar-button");
			// Walk up the tree
			let target = e.target as Element;
			while (target != null && target != sidebar && target != sidebarButton) {
				target = target.parentNode as Element;
			}

			if (target == null) {
				if (props.visible) {
					props.setVisible(false);
				}
			}
		});
	});

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

	return (
		<Show when={props.visible}>
			<div id="sidebar">
				<div id="sidebar-items">
					<SidebarItem
						href="/"
						location={props.location}
						selectedPath="/"
						color="var(--fg3)"
						closeSidebar={() => props.setVisible(false)}
					>
						<div style="margin-top:0.15rem;margin-right:-0.2rem;color:var(--fg2)">
							<Home />
						</div>
						<div>Home</div>
					</SidebarItem>
					<SidebarItem
						href="/packages/0"
						location={props.location}
						selectedPathStart="/packages"
						color="var(--package)"
						closeSidebar={() => props.setVisible(false)}
					>
						<div style="margin-top:0.3rem;margin-right:-0.2rem;color:var(--package)">
							<Box />
						</div>
						<div>Packages</div>
					</SidebarItem>
					<SidebarItem
						href="/plugins"
						location={props.location}
						selectedPathStart="/plugins"
						color="var(--plugin)"
						closeSidebar={() => props.setVisible(false)}
					>
						<div style="margin-top:0.1rem;margin-right:-0.2rem;color:var(--plugin)">
							<Jigsaw />
						</div>
						<div>Plugins</div>
					</SidebarItem>
					<SidebarItem
						href="/docs"
						location={props.location}
						selectedPathStart="/docs"
						color="var(--profile)"
						closeSidebar={() => props.setVisible(false)}
					>
						<div style="margin-top:0.3rem;margin-right:-0.2rem;color:var(--profile)">
							<Menu />
						</div>
						<div>Documentation</div>
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
				</div>
			</div>
		</Show>
	);
}

export interface SidebarProps {
	visible: boolean;
	setVisible: (visible: boolean) => void;
	location: Location;
}

function SidebarItem(props: SidebarItemProps) {
	const selected = () => {
		if (props.selectedPath != undefined) {
			return props.location.pathname == props.selectedPath;
		}
		if (props.selectedPathStart != undefined) {
			return props.location.pathname.startsWith(props.selectedPathStart);
		}

		return false;
	};
	return (
		<a
			class={`cont link sidebar-item ${selected() ? "selected" : ""}`}
			href={props.href}
			style={`border-right-color:${props.color}`}
			onclick={() => props.closeSidebar()}
			innerHTML={props.innerhtml}
		>
			{props.children}
		</a>
	);
}

interface SidebarItemProps {
	children?: JSX.Element;
	innerhtml?: string;
	href: string;
	location: Location;
	// What the current URL should equal to select this item
	selectedPath?: string;
	// What the current URL should start with to select this item
	selectedPathStart?: string;
	color: string;
	closeSidebar: () => void;
}

interface PluginSidebarButton {
	html: string;
	href: string;
	selected_url?: string;
	selected_url_start?: string;
	color: string;
}
