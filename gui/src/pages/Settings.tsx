import { invoke } from "@tauri-apps/api/core";
import { createEffect, createResource, createSignal, Show } from "solid-js";
import { Theme } from "../types";
import Tip from "../components/dialog/Tip";
import InlineSelect from "../components/input/select/InlineSelect";
import { FooterData } from "../App";
import { FooterMode } from "../components/navigation/Footer";
import { errorToast, successToast } from "../components/dialog/Toasts";
import { emit } from "@tauri-apps/api/event";
import IconTextButton from "../components/input/button/IconTextButton";
import { Folder } from "../icons";

export default function Settings(props: SettingsProps) {
	createEffect(() => {
		props.setFooterData({
			mode: FooterMode.SaveSettings,
			selectedItem: undefined,
			action: saveSettings,
		});
	});

	let [settings, _] = createResource(
		async () => (await invoke("get_settings")) as LauncherSettings
	);

	let [availableThemes, __] = createResource(async () => {
		let themes = (await invoke("get_themes")) as Theme[];
		themes = [
			{
				id: "dark",
				name: "Dark",
				description: "Default dark theme",
				css: "",
				color: "var(--bg4)",
			} as Theme,
			{
				id: "light",
				name: "Light",
				description: "Default light theme",
				css: "",
				color: "var(--fg2)",
			} as Theme,
		].concat(themes);

		return themes;
	});

	let [theme, setTheme] = createSignal<string>("dark");

	createEffect(() => {
		if (settings() == undefined) {
			return;
		}

		let theme = settings()!.selected_theme;
		setTheme(theme == undefined ? "dark" : theme);
	});

	async function saveSettings() {
		if (settings() == undefined) {
			return;
		}

		let newSettings: LauncherSettings = {
			selected_theme: theme(),
		};

		try {
			await invoke("write_settings", { settings: newSettings });
			successToast("Changes saved");

			props.setFooterData({
				mode: FooterMode.SaveSettings,
				selectedItem: undefined,
				action: saveSettings,
			});

			emit("update_theme", theme());
		} catch (e) {
			errorToast("Failed to save: " + e);
		}
	}

	function setDirty() {
		props.setFooterData({
			mode: FooterMode.SaveSettings,
			selectedItem: "",
			action: saveSettings,
		});
	}

	return (
		<div id="plugins">
			<h1 class="noselect">Settings</h1>
			<div class="cont fullwidth">
				<div class="cont fields">
					<div class="cont start label">
						<label for="theme">THEME</label>
					</div>
					<Tip tip="The launcher theme to use" fullwidth>
						<Show when={availableThemes() != undefined}>
							<InlineSelect
								onChange={(x) => {
									setTheme(x as string);
									setDirty();
								}}
								selected={theme()}
								options={availableThemes()!.map((theme) => {
									return {
										value: theme.id,
										contents: <div>{theme.name}</div>,
										tip: theme.description,
										color: theme.color,
										selectedTextColor: "var(--fg)",
									};
								})}
								columns={1}
								allowEmpty={false}
								connected={false}
							/>
						</Show>
					</Tip>
					<Tip
						tip="Open the folder where Nitrolaunch stores its instances and data"
						fullwidth
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
			</div>
			<br />
			<br />
			<br />
			<br />
		</div>
	);
}

export interface SettingsProps {
	setFooterData: (data: FooterData) => void;
}

// Global launcher settings
export interface LauncherSettings {
	selected_theme?: string;
}
