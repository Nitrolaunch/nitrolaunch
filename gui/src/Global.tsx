import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { createMemo, createResource, createSignal, For, onCleanup } from "solid-js";
import { Theme } from "./types";
import { LauncherSettings } from "./pages/Settings";
import { errorToast } from "./components/dialog/Toasts";

// Global / startup functions like loading plugins, startup messages, and themes
export default function Global(props: GlobalProps) {
	let [baseTheme, setBaseTheme] = createSignal<string>("dark");
	let [overlayThemes, setOverlayThemes] = createSignal<string[]>([]);

	let [unlisten, setUnlisten] = createSignal<UnlistenFn>(() => { });
	let [availableThemes, availableThemesMethods] = createResource(async () => {
		let unlisten = await listen("update_theme", () => {
			availableThemesMethods.refetch();
		});

		setUnlisten(() => unlisten);

		try {
			let [availableThemes, settings] = (await Promise.all([
				invoke("get_themes"),
				invoke("get_settings"),
			])) as [Theme[], LauncherSettings];

			if (settings.base_theme != undefined) {
				setBaseTheme(settings.base_theme);
			}
			setOverlayThemes(settings.overlay_themes);

			return availableThemes;
		} catch (e) {
			errorToast("Failed to load theme");
		}
	});

	onCleanup(() => {
		unlisten();
	});

	function getThemeElement(theme: string) {
		if (theme == "dark" || theme == "light") {
			return `<link rel="stylesheet" href="/themes/${theme}.css" />`;
		} else {
			if (
				availableThemes() != undefined &&
				availableThemes()!.some((x) => x.id == theme)
			) {
				let css = availableThemes()!.find((x) => x.id == theme)!.css;
				return `<style>${css}</style>`;
			} else {
				return `<link rel="stylesheet" href="/themes/${theme}.css" />`;
			}
		}
	}

	let baseElement = createMemo(() => getThemeElement(baseTheme()));
	let overlayElements = createMemo(() => overlayThemes().map(getThemeElement));

	return (
		<div style="position:absolute">
			<div innerHTML={baseElement()}></div>
			<For each={overlayElements()}>
				{(elem) => <div innerHTML={elem}></div>}
			</For>
		</div>
	);
}

export interface GlobalProps { }

export interface UpdateThemeEvent {
	base_theme: string;
	overlay_themes: string[];
}
