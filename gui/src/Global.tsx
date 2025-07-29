import { invoke } from "@tauri-apps/api";
import { Event, listen, UnlistenFn } from "@tauri-apps/api/event";
import { createResource, createSignal, onCleanup } from "solid-js";
import { Theme } from "./types";
import { LauncherSettings } from "./pages/Settings";
import { errorToast } from "./components/dialog/Toasts";
import { Dynamic } from "solid-js/web";

// Global / startup functions like loading plugins, startup messages, and themes
export default function Global(props: GlobalProps) {
	let [selectedTheme, setSelectedTheme] = createSignal<string>("dark");

	let [unlisten, setUnlisten] = createSignal<UnlistenFn>(() => {});
	let [availableThemes, _] = createResource(async () => {
		let unlisten = await listen("update_theme", (e: Event<string>) => {
			setSelectedTheme(e.payload);
		});

		setUnlisten(() => unlisten);

		try {
			let [availableThemes, settings] = (await Promise.all([
				invoke("get_themes"),
				invoke("get_settings"),
			])) as [Theme[], LauncherSettings];

			if (settings.selected_theme != undefined) {
				setSelectedTheme(settings.selected_theme);
			}

			return availableThemes;
		} catch (e) {
			errorToast("Failed to load theme");
		}
	});

	onCleanup(() => {
		unlisten();
	});

	let styleElement = () => {
		if (selectedTheme() == "dark" || selectedTheme() == "light") {
			return <link rel="stylesheet" href={`/themes/${selectedTheme()}.css`} />;
		} else {
			if (
				availableThemes() != undefined &&
				availableThemes()!.some((x) => x.id == selectedTheme())
			) {
				let css = availableThemes()!.find((x) => x.id == selectedTheme())!.css;
				return <style innerText={css}></style>;
			} else {
				return (
					<link rel="stylesheet" href={`/themes/${selectedTheme()}.css`} />
				);
			}
		}
	};

	return (
		<div style="position:absolute">
			<Dynamic component={styleElement} />
		</div>
	);
}

export interface GlobalProps {}
