import { invoke } from "@tauri-apps/api/core";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	Show,
} from "solid-js";
import { Theme } from "../types";
import Tip from "../components/dialog/Tip";
import InlineSelect from "../components/input/select/InlineSelect";
import { errorToast, successToast } from "../components/dialog/Toasts";
import { emit } from "@tauri-apps/api/event";
import IconTextButton from "../components/input/button/IconTextButton";
import { Check, Delete, Folder, Gear, Jigsaw, Text } from "../icons";
import Modal, { ModalButton } from "../components/dialog/Modal";
import FloatingTabs from "../components/input/select/FloatingTabs";
import ApplicationLog from "../components/ApplicationLog";
import { ControlData } from "../components/input/Control";
import { ControlledConfig } from "./instance/read_write";
import ControlSections from "../components/input/ControlSections";

export default function Settings(props: SettingsProps) {
	let [settings, settingsMethods] = createResource(
		async () => (await invoke("get_settings")) as LauncherSettings,
	);

	let [tab, setTab] = createSignal("general");

	let [availableThemes, __] = createResource(async () => {
		let themes: Theme[] = [];
		try {
			themes = (await invoke("get_themes")) as Theme[];
		} catch (e) {
			errorToast("Failed to load themes: " + e);
		}
		themes = [
			{
				id: "dark",
				name: "Dark",
				description: "Default dark theme",
				type: "base",
				css: "",
				color: "var(--bg4)",
			} as Theme,
			{
				id: "light",
				name: "Light",
				description: "Default light theme",
				type: "base",
				css: "",
				color: "var(--fg2)",
			} as Theme,
		].concat(themes);

		return themes;
	});

	let [isDirty, setIsDirty] = createSignal(false);

	createEffect(() => {
		if (props.isVisible) {
			setIsDirty(false);
			settingsMethods.refetch();
		}
	});

	let [baseTheme, setBaseTheme] = createSignal<string>("dark");
	let [overlayThemes, setOverlayThemes] = createSignal<string[]>([]);

	let pluginConfig: { [plugin: string]: ControlledConfig } = {};
	createEffect(async () => {
		settings();

		try {
			let config = (await invoke("get_plugin_config")) as {
				[key: string]: any;
			};
			pluginConfig = {};
			for (let key in config) {
				pluginConfig[key] = new ControlledConfig(config[key]);
			}
		} catch (e) {
			errorToast("Failed to load plugin config: " + e);
		}
	});

	let [pluginControls, pluginControlsMethods] = createResource(
		async () => {
			try {
				let controls = (await invoke("get_plugin_config_controls")) as {
					[plugin: string]: ControlData[];
				};

				return controls;
			} catch (e) {
				errorToast("Failed to load controls: " + e);
				return {};
			}
		},
		{ initialValue: {} },
	);

	createEffect(() => {
		if (settings() == undefined) {
			return;
		}

		let baseTheme = settings()!.base_theme;
		setBaseTheme(baseTheme == undefined ? "dark" : baseTheme);
		setOverlayThemes(settings()!.overlay_themes);
		pluginControlsMethods.refetch();
	});

	async function saveSettings() {
		if (settings() == undefined) {
			return;
		}

		let newSettings: LauncherSettings = {
			base_theme: baseTheme(),
			overlay_themes: overlayThemes(),
		};

		try {
			await Promise.all([
				invoke("write_settings", { settings: newSettings }),
				savePluginConfig(),
			]);
			successToast("Changes saved");

			setIsDirty(false);

			emit("update_theme");
		} catch (e) {
			errorToast("Failed to save: " + e);
		}
	}

	async function savePluginConfig() {
		let configs: { [key: string]: any } = {};
		for (let plugin in pluginConfig) {
			let config = pluginConfig[plugin];
			if (pluginControls()[plugin] != undefined) {
				config.cleanup(pluginControls()[plugin]);
			}
			configs[plugin] = config.fields;
		}

		await invoke("write_plugin_config", { config: configs });
	}

	let buttons = () => {
		let saveButton: ModalButton = {
			text: "Save",
			icon: Check,
			color: isDirty() ? "var(--template)" : "var(--bg3)",
			bgColor: isDirty() ? "var(--templatebg)" : undefined,
			onClick: saveSettings,
		};

		return [
			{
				text: "Cancel",
				icon: Delete,
				onClick: props.onClose,
			},
			saveButton,
		] as ModalButton[];
	};

	return (
		<Modal
			width="60rem"
			height="35rem"
			titleIcon={Gear}
			title="Settings"
			visible={props.isVisible}
			onClose={props.onClose}
			buttons={buttons()}
		>
			<div class="cont fullwidth">
				<FloatingTabs
					tabs={[
						{
							id: "general",
							title: "General",
							icon: Gear,
							color: "var(--instance)",
							bgColor: "var(--instancebg)",
						},
						{
							id: "logs",
							title: "Logs",
							icon: Text,
							color: "var(--template)",
							bgColor: "var(--templatebg)",
						},
						{
							id: "plugins",
							title: "Plugins",
							icon: Jigsaw,
							color: "var(--plugin)",
							bgColor: "var(--pluginbg)",
						},
					]}
					selectedTab={tab()}
					setTab={setTab}
				/>
			</div>
			<div class="cont fullwidth">
				<Show when={tab() == "general"}>
					<div class="cont fields">
						<div class="cont start label">
							<label for="theme">BASE THEME</label>
						</div>
						<Show when={availableThemes() != undefined}>
							<InlineSelect
								onChange={(x) => {
									setBaseTheme(x as string);
									setIsDirty(true);
								}}
								selected={baseTheme()}
								options={availableThemes()!
									.filter((x) => x.type == "base")
									.map((theme) => {
										return {
											value: theme.id,
											contents: <div>{theme.name}</div>,
											tip: theme.description,
											color: theme.color,
											selectedTextColor: "var(--fg)",
										};
									})}
								columns={3}
								allowEmpty={false}
								connected={false}
							/>
						</Show>
						<div class="cont start label">
							<label for="theme">OVERLAY THEMES</label>
						</div>
						<Show when={availableThemes() != undefined}>
							<InlineSelect
								onChangeMulti={(x) => {
									setOverlayThemes(x as string[]);
									setIsDirty(true);
								}}
								selected={overlayThemes()}
								options={availableThemes()!
									.filter((x) => x.type == "overlay")
									.map((theme) => {
										return {
											value: theme.id,
											contents: <div>{theme.name}</div>,
											tip: theme.description,
											color: theme.color,
											selectedTextColor: "var(--fg)",
										};
									})}
								columns={3}
								allowEmpty={false}
								connected={false}
							/>
						</Show>
						<Tip
							tip="Open the folder where Nitrolaunch stores its instances and data"
							side="top"
						>
							<div class="cont">
								<IconTextButton
									icon={Folder}
									size="1rem"
									text="Open data folder"
									onClick={() => invoke("open_data_dir")}
								/>
							</div>
						</Tip>
					</div>
				</Show>
				<Show when={tab() == "logs"}>
					<div class="cont col fullwidth">
						<ApplicationLog />
					</div>
				</Show>
				<Show when={tab() == "plugins"}>
					<div class="cont col fullwidth">
						<For each={Object.keys(pluginControls())}>
							{(plugin) => (
								<ControlSections
									controls={pluginControls()[plugin]}
									getInitialValue={(id) => {
										if (pluginConfig[plugin] == undefined) {
											return undefined;
										} else {
											return pluginConfig[plugin].getControl(id);
										}
									}}
									setValue={(id, value) => {
										if (pluginConfig[plugin] == undefined) {
											pluginConfig[plugin] = new ControlledConfig({});
										}

										pluginConfig[plugin].setControl(id, value);
										setIsDirty(true);
									}}
								/>
							)}
						</For>
					</div>
				</Show>
			</div>
		</Modal>
	);
}

export interface SettingsProps {
	isVisible: boolean;
	onClose: () => void;
}

// Global launcher settings
export interface LauncherSettings {
	base_theme?: string;
	overlay_themes: string[];
}
