import { createSignal, JSX } from "solid-js";
import { AngleLeft, AngleRight, Box, Home, Jigsaw, Menu } from "../../icons";
import IconButton from "../input/IconButton";
import "./NavBar.css";
import { Location } from "@solidjs/router";
import Toasts from "../dialog/Toasts";

export default function NavBar(props: NavBarProps) {
	return (
		<>
			{/* Gap used to move page content down so that it starts below the navbar */}
			<div id="navbar-gap"></div>
			<div id="navbar">
				<div id="navbar-container">
					<div class="cont navbar-item" id="navbar-left">
						<div id="sidebar-button">
							<IconButton
								icon={Menu}
								size="28px"
								color="var(--bg)"
								selectedColor="var(--accent)"
								onClick={props.onSidebarToggle}
								selected={false}
								circle
								hoverBackground="var(--bg3)"
							/>
						</div>
						<IconButton
							icon={AngleLeft}
							size="28px"
							color="var(--bg)"
							selectedColor="var(--accent)"
							onClick={() => {
								history.back();
							}}
							selected={false}
							circle
							hoverBackground="var(--bg3)"
						/>
						<IconButton
							icon={AngleRight}
							size="28px"
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
					<div class="navbar-item" id="navbar-buttons">
						<NavbarButton
							icon={<Home />}
							text="Home"
							href="/"
							selectedPath={["/"]}
							selectedPathStart={[
								"/instance",
								"/instance_config",
								"/profile_config",
								"/global_profile_config",
								"create_instance",
								"create_profile",
							]}
							color="var(--instance)"
							backgroundColor="var(--instancebg)"
							location={props.location}
						/>
						<NavbarButton
							icon={<Box />}
							text="Packages"
							href="/packages/0"
							selectedPathStart={["/packages"]}
							color="var(--package)"
							backgroundColor="var(--packagebg)"
							location={props.location}
						/>
						<NavbarButton
							icon={<Jigsaw />}
							text="Plugins"
							href="/plugins"
							selectedPathStart={["/plugins"]}
							color="var(--plugin)"
							backgroundColor="var(--pluginbg)"
							location={props.location}
						/>
					</div>
					<h3 class="cont bubble-hover navbar-item">
						<a href="/" class="cont link bold" title="Return to the homepage">
							<img src="/Logo.png" style="width:1.5rem;border-radius:var(--round)" class="input-shadow" />
							NITRO
						</a>
					</h3>
					<div class="cont end navbar-item" id="navbar-right">
						<Toasts />
					</div>
				</div>
			</div>
		</>
	);
}

export interface NavBarProps {
	onSidebarToggle: () => void;
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
		>
			{props.icon}
			<div class="navbar-button-text">{props.text}</div>
		</a>
	);
}

interface NavbarButtonProps {
	icon: JSX.Element;
	text: string;
	href: string;
	location: Location;
	// What the current URL should equal to select this item
	selectedPath?: string[];
	// What the current URL should start with to select this item
	selectedPathStart?: string[];
	color: string;
	backgroundColor: string;
}
