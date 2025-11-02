import { createSignal, JSX } from "solid-js";
import { ArrowLeft, ArrowRight, Home, Honeycomb, Jigsaw, Menu } from "../../icons";
import IconButton from "../input/button/IconButton";
import "./NavBar.css";
import { Location } from "@solidjs/router";
import Toasts from "../dialog/Toasts";
import Icon, { HasWidthHeight } from "../Icon";
import UserWidget from "../user/UserWidget";

export default function NavBar(props: NavBarProps) {
	return (
		<>
			{/* Gap used to move page content down so that it starts below the navbar */}
			<div id="navbar-gap"></div>
			<div id="navbar">
				<div id="navbar-container">
					<div class="split3 fullwidth navbar-item" id="navbar-left">
						<div class="cont" id="sidebar-button">
							<IconButton
								icon={Menu}
								size="1.8rem"
								color="var(--bg)"
								selectedColor="var(--accent)"
								onClick={props.onSidebarToggle}
								selected={false}
								circle
								hoverBackground="var(--bg3)"
							/>
						</div>
						<div class="cont">
							<IconButton
								icon={ArrowLeft}
								size="1.8rem"
								color="var(--bg)"
								selectedColor="var(--accent)"
								onClick={() => {
									history.back();
								}}
								selected={false}
								circle
								hoverBackground="var(--bg3)"
							/>
						</div>
						<div class="cont">
							<IconButton
								icon={ArrowRight}
								size="1.8rem"
								color="var(--bg)"
								selectedColor="var(--accent)"
								onClick={() => {
									history.forward();
								}}
								selected={false}
								circle
								hoverBackground="var(--bg3)"
							/>
						</div>
					</div>
					<div class="navbar-item" id="navbar-buttons">
						<NavbarButton
							icon={Home}
							text="Home"
							href="/"
							selectedPath={["/"]}
							selectedPathStart={[
								"/instance",
								"/instance_config",
								"/profile_config",
								"/base_profile_config",
								"create_instance",
								"create_profile",
							]}
							color="var(--instance)"
							backgroundColor="var(--instancebg)"
							location={props.location}
							onClick={props.onSidebarClose}
						/>
						<NavbarButton
							icon={Honeycomb}
							text="Packages"
							href="/packages/0"
							selectedPathStart={["/packages"]}
							color="var(--package)"
							backgroundColor="var(--packagebg)"
							location={props.location}
							onClick={props.onSidebarClose}
						/>
						<NavbarButton
							icon={Jigsaw}
							text="Plugins"
							href="/plugins"
							selectedPathStart={["/plugins"]}
							color="var(--plugin)"
							backgroundColor="var(--pluginbg)"
							location={props.location}
							onClick={props.onSidebarClose}
						/>
					</div>
					<h3 class="cont bubble-hover navbar-item">
						<a href="/" class="cont link bold" title="Return to the homepage" style="position:relative">
							<img src="/Logo.png" style="width:1.5rem;border-radius:var(--round)" class="input-shadow" />
							<div id="logo-text">NITRO</div>
							<div class="cont" id="beta-indicator">
								BETA
							</div>
						</a>
					</h3>
					<div class="cont end navbar-item" id="navbar-right">
						<UserWidget onSelect={props.onSelectUser} />
						<Toasts />
					</div>
				</div>
			</div>
		</>
	);
}

export interface NavBarProps {
	onSidebarToggle: () => void;
	onSidebarClose: () => void;
	onSelectUser: (user: string) => void;
	location: Location;
}

function NavbarButton(props: NavbarButtonProps) {
	let [isHovered, setIsHovered] = createSignal(false);

	const selected = () => {
		if (props.selectedPath != undefined) {
			for (let path of props.selectedPath) {
				if (props.location.pathname == path) {
					return true;
				}
			}
		}
		if (props.selectedPathStart != undefined) {
			for (let path of props.selectedPathStart) {
				if (props.location.pathname.startsWith(path)) {
					return true;
				}
			}
		}

		return false;
	};

	let color = () => (selected() ? props.color : "var(--fg)");
	let borderColor = () => (selected() || isHovered() ? props.color : "");

	return (
		<a
			class={`cont link navbar-button bubble-hover ${selected() ? "selected" : ""}`}
			href={props.href}
			style={`color:${color()};background-color:${selected() ? props.backgroundColor : "var(--bg)"
				};border-color:${borderColor()}`}
			onmouseenter={() => setIsHovered(true)}
			onmouseleave={() => setIsHovered(false)}
			onclick={props.onClick}
		>
			<Icon icon={props.icon} size="1rem" />
			<div class="navbar-button-text">{props.text}</div>
		</a>
	);
}

interface NavbarButtonProps {
	icon: (props: HasWidthHeight) => JSX.Element;
	text: string;
	href: string;
	location: Location;
	// What the current URL should equal to select this item
	selectedPath?: string[];
	// What the current URL should start with to select this item
	selectedPathStart?: string[];
	color: string;
	backgroundColor: string;
	onClick: () => void;
}
