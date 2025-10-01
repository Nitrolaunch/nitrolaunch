import { invoke } from "@tauri-apps/api";
import { WebviewWindow } from "@tauri-apps/api/window";
import { errorToast } from "./components/dialog/Toasts";

// Loads plugins on a page
export function loadPagePlugins(page: string, object?: string) {
	try {
		invoke("get_page_inject_script", { page: page, object: object }).then(
			(script) => {
				let script2 = script as string;
				eval(script2);
				console.log("Page plugins loaded successfully");
			},
			(e) => {
				console.error("Failed to load page plugins: " + e);
			}
		);

	} catch (e) {
		errorToast("Failed to load page plugins: " + e);
	}
	setupPluginFunctions();
}

export function setupPluginFunctions() {
	let global = window as any;
	global.tauriInvoke = async (command: any, args: any) => {
		await invoke(command, args);
	};
	global.TauriWindow = WebviewWindow;
	global.customAction = async (plugin: string, action: string, payload: any) => {
		return await invoke("run_custom_action", { plugin: plugin, action: action, payload: payload });
	}
}
