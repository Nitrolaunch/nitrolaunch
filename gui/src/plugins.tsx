import { clipboard, invoke } from "@tauri-apps/api";
import { WebviewWindow } from "@tauri-apps/api/window";
import { errorToast, messageToast, successToast, warningToast } from "./components/dialog/Toasts";
import { Option } from "./components/input/select/Dropdown";
import IconAndText from "./components/utility/IconAndText";
import { HTMLIcon } from "./components/Icon";
import { clearInputError, inputError } from "./errors";
import { emit } from "@tauri-apps/api/event";
import { sanitizeInstanceId } from "./pages/instance/InstanceConfig";
import { updateInstanceList } from "./pages/instance/InstanceList";

// Loads plugins on a page
export function loadPagePlugins(page: string, object?: string) {
	try {
		invoke("get_page_inject_script", { page: page, object: object }).then(
			(script) => {
				let script2 = script as string;
				eval(script2);
				console.log(`Plugins loaded successfully for page ${page}`);
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
	global.copyToClipboard = clipboard.writeText;

	global.customAction = async (plugin: string, action: string, payload: any) => {
		return await invoke("run_custom_action", { plugin: plugin, action: action, payload: payload });
	}
	global.setModal = global.__setPluginModalContents;

	global.globalInterval = (id: string, f: () => void, interval: number) => {
		let id2 = `interval${id}`;
		if (global[id2] != undefined) {
			clearInterval(global[id2]);
		}

		global[id2] = setInterval(f, interval);
	};

	global.showMessageToast = messageToast;
	global.showSuccessToast = successToast;
	global.showWarningToast = warningToast;
	global.showErrorToast = errorToast;

	global.startTask = async (id: string) => await emit("nitro_output_create_task", id);
	global.endTask = async (id: string) => await emit("nitro_output_finish_task", id);

	global.inputError = inputError;
	global.clearInputError = clearInputError;
	global.sanitizeInstanceID = sanitizeInstanceId;

	global.updateInstanceList = updateInstanceList;
}

export async function getDropdownButtons(location: DropdownButtonLocation): Promise<PluginDropdownButton[]> {
	try {
		return await invoke("get_dropdown_buttons", { location: location });
	} catch (e) {
		console.error("Failed to get dropdown buttons: " + e);
		return [];
	}
}

export type DropdownButtonLocation = "add_template_or_instance" | "instance_launch" | "instance_update" | "instance_more_options";

export interface PluginDropdownButton {
	plugin: string;
	location: DropdownButtonLocation;
	icon: string;
	text: string;
	color?: string;
	tip?: string;
	action?: string;
	on_click?: string;
}

export function dropdownButtonToOption(button: PluginDropdownButton): Option {
	return {
		value: button.action == undefined ? button.on_click : `custom_${button.plugin}-${button.action}`,
		contents: <IconAndText icon={HTMLIcon(button.icon)} text={button.text} />,
		color: button.color,
		tip: button.tip,
	};
}

export function runDropdownButtonClick(selection: string) {
	console.log(selection);
	if (selection.startsWith("custom_")) {
		let data = selection!.replace("custom_", "");
		let split = data.split("-");
		let plugin = split[0];
		let action = split[1];
		(window as any).customAction(plugin, action, null);
	} else {
		eval(selection);
	}
}
