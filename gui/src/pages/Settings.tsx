import { invoke } from "@tauri-apps/api/core";
import { createEffect, createResource, createSignal, Show } from "solid-js";
import { Theme } from "../types";
import Tip from "../components/dialog/Tip";
import InlineSelect from "../components/input/select/InlineSelect";
import { errorToast, successToast } from "../components/dialog/Toasts";
import { emit } from "@tauri-apps/api/event";
import IconTextButton from "../components/input/button/IconTextButton";
import { Check, Delete, Folder, Gear } from "../icons";
import Modal, { ModalButton } from "../components/dialog/Modal";

export default function Settings(props: SettingsProps) {
	let [settings, settingsMethods] = createResource(
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

	let [isDirty, setIsDirty] = createSignal(false);

	createEffect(() => {
		if (props.isVisible) {
			setIsDirty(false);
			settingsMethods.refetch();
		}
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

			setIsDirty(false);

			emit("update_theme", theme());
		} catch (e) {
			errorToast("Failed to save: " + e);
		}
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
				<div class="cont fields">
					<div class="cont start label">
						<label for="theme">THEME</label>
					</div>
					<Show when={availableThemes() != undefined}>
						<InlineSelect
							onChange={(x) => {
								setTheme(x as string);
								setIsDirty(true);
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
	selected_theme?: string;
}
