import { Router, Route, Location } from "@solidjs/router";
import "./App.css";
import LaunchPage from "./pages/instance/InstanceList";
import NavBar from "./components/navigation/NavBar";
import { createSignal, ErrorBoundary, onMount, Show } from "solid-js";
import InstanceConfig from "./pages/instance/InstanceConfig";
import BrowsePackages from "./pages/package/BrowsePackages";
import ViewPackage from "./pages/package/ViewPackage";
import Sidebar from "./components/navigation/Sidebar";
import Plugins from "./pages/plugin/Plugins";
import Docs from "./pages/Docs";
import { loadPagePlugins } from "./plugins";
import { listen } from "@tauri-apps/api/event";
import CustomPluginPage from "./pages/CustomPluginPage";
import Footer, { FooterMode } from "./components/navigation/Footer";
import { InstanceConfigMode } from "./pages/instance/read_write";
import InstanceInfo from "./pages/instance/InstanceInfo";
import UserPage from "./pages/user/UserPage";
import Global from "./Global";
import Settings from "./pages/Settings";
import "./components/package/PackageDescription.css";
import ModalBase from "./components/dialog/ModalBase";
import WelcomePrompt from "./components/dialog/WelcomePrompt";
import { invoke } from "@tauri-apps/api/core";

export default function App() {
	const [footerData, setFooterData] = createSignal<FooterData>({
		selectedItem: undefined,
		mode: FooterMode.Instance,
		action: () => {},
	});

	let [selectedUser, setSelectedUser] = createSignal<string>();

	// Window refresh logic
	let [showUi, setShowUi] = createSignal(true);
	listen("refresh_window", () => {
		setShowUi(false);
		setShowUi(true);
	});

	return (
		<Show when={showUi()}>
			<Router
				root={({ children, location }) => (
					<Layout
						children={children}
						location={location}
						footerData={footerData()}
						onSelectUser={setSelectedUser}
						selectedUser={selectedUser()}
					/>
				)}
			>
				<Route
					path="/"
					component={() => <LaunchPage setFooterData={setFooterData} />}
				/>
				<Route
					path="/instance/:instanceId"
					component={() => <InstanceInfo setFooterData={setFooterData} />}
				/>
				<Route
					path="/instance_config/:instanceId"
					component={() => (
						<InstanceConfig
							mode={InstanceConfigMode.Instance}
							creating={false}
							setFooterData={setFooterData}
						/>
					)}
				/>
				<Route
					path="/template_config/:TemplateID"
					component={() => (
						<InstanceConfig
							mode={InstanceConfigMode.Template}
							creating={false}
							setFooterData={setFooterData}
						/>
					)}
				/>
				<Route
					path="/create_instance"
					component={() => (
						<InstanceConfig
							mode={InstanceConfigMode.Instance}
							creating={true}
							setFooterData={setFooterData}
						/>
					)}
				/>
				<Route
					path="/create_template"
					component={() => (
						<InstanceConfig
							mode={InstanceConfigMode.Template}
							creating={true}
							setFooterData={setFooterData}
						/>
					)}
				/>
				<Route
					path="/base_template_config"
					component={() => (
						<InstanceConfig
							mode={InstanceConfigMode.GlobalTemplate}
							creating={false}
							setFooterData={setFooterData}
						/>
					)}
				/>
				<Route
					path="/packages/:page"
					component={() => <BrowsePackages setFooterData={setFooterData} />}
				/>
				<Route
					path="/packages/package/:id"
					component={() => <ViewPackage setFooterData={setFooterData} />}
				/>
				<Route path="/users/:userId" component={() => <UserPage />} />
				<Route path="/plugins" component={() => <Plugins />} />
				<Route
					path="/settings"
					component={() => <Settings setFooterData={setFooterData} />}
				/>
				<Route path="/docs" component={() => <Docs />} />
				<Route path="/custom/:page" component={() => <CustomPluginPage />} />
			</Router>
		</Show>
	);
}

function Layout(props: LayoutProps) {
	let [showSidebar, setShowSidebar] = createSignal(false);
	// Modal for plugins to use
	let [pluginModalContents, setPluginModalContents] = createSignal<
		string | undefined
	>();
	let [showWelcomePrompt, setShowWelcomePrompt] = createSignal(true);

	(window as any).__setPluginModalContents = (x: any) => {
		setPluginModalContents(x);
		console.log("Ok");
	};

	onMount(() => loadPagePlugins(""));

	onMount(async () => {
		try {
			let isFirstLaunch = await invoke("get_is_first_launch");
			if (isFirstLaunch) {
				setShowWelcomePrompt(true);
			}
		} catch (e) {
			console.error(e);
		}
	});

	// Fix for Webkitgtk scrolling
	onMount(async() => {
		if (await invoke("custom_scrollbar_needed")) {
			let elem = document.getElementById("root")!;
			elem.style.overflowX = "scroll";
			elem.style.maxHeight = "100vh";
		}
	})

	return (
		<>
			<Global />
			<NavBar
				onSidebarToggle={() => setShowSidebar(!showSidebar())}
				onSidebarClose={() => setShowSidebar(false)}
				onSelectUser={props.onSelectUser}
				location={props.location}
			/>
			<ErrorBoundary
				fallback={
					<div>An error occurred in the page. Please report this issue.</div>
				}
			>
				{props.children}
			</ErrorBoundary>
			<Sidebar
				visible={showSidebar()}
				location={props.location}
				setVisible={setShowSidebar}
				onSelectUser={props.onSelectUser}
			/>
			<Footer
				selectedItem={props.footerData.selectedItem}
				mode={props.footerData.mode}
				selectedUser={props.selectedUser}
				action={props.footerData.action}
				itemFromPlugin={props.footerData.fromPlugin}
				selectedPackageGallery={props.footerData.selectedPackageGallery}
			/>
			<ModalBase
				visible={pluginModalContents() != undefined}
				onClose={() => setPluginModalContents(undefined)}
				width="40rem"
			>
				<div class="cont col fullwidth" innerHTML={pluginModalContents()}></div>
			</ModalBase>
			<WelcomePrompt
				visible={showWelcomePrompt()}
				onClose={() => setShowWelcomePrompt(false)}
			/>
		</>
	);
}

interface LayoutProps {
	children: any;
	location: Location;
	footerData: FooterData;
	selectedUser?: string;
	onSelectUser: (user: string) => void;
}

export interface FooterData {
	selectedItem?: string;
	mode: FooterMode;
	action: () => void;
	// Whether a selected instance or template was created by a plugin
	fromPlugin?: boolean;
	selectedPackageGallery?: string[];
}
