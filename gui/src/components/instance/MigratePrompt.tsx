import { createResource, createSignal, Match, Switch } from "solid-js";
import ModalBase from "../dialog/ModalBase";
import { invoke } from "@tauri-apps/api/core";
import "./TemplateDeletePrompt.css";
import InlineSelect from "../input/select/InlineSelect";
import IconTextButton from "../input/button/IconTextButton";
import { Cycle } from "../../icons";
import { errorToast, successToast } from "../dialog/Toasts";
import { clearInputError, inputError } from "../../errors";
import Icon from "../Icon";
import Tip from "../dialog/Tip";
import { InstanceTransferFormat } from "./InstanceTransferPrompt";
import { updateInstanceList } from "../../pages/instance/InstanceList";

export default function MigratePrompt(
	props: MigratePromptProps
) {
	return (
		<ModalBase visible={props.visible} onClose={props.onClose} width="25rem">
			<MigratePromptContents {...props} />
		</ModalBase>
	);
}

// Inner contents of the migrate modal. Split into its own function so we can use it in the welcome prompt too.
export function MigratePromptContents(props: MigratePromptProps) {
	let [formats, _] = createResource(
		() => props.visible,
		async () => {
			let formats = (await invoke(
				"get_instance_transfer_formats"
			)) as InstanceTransferFormat[];

			// Filter out formats that don't support migration
			formats = formats.filter((format) => format.migrate != undefined);

			return formats;
		},
		{ initialValue: [] }
	);

	let [selectedFormat, setSelectedFormat] = createSignal<string | undefined>();

	return <div class="cont col" style="padding:2rem">
		<div class="cont bold">
			<Icon icon={Cycle} size="1rem" />
			Migrate from Launcher
		</div>
		<div></div>
		<div class="cont fields" style="width:100%">
			<div class="cont start label">
				<label>FORMAT</label>
			</div>
			<Tip
				fullwidth
				tip="The launcher to migrate from"
			>
				<div class="fullwidth" id="instance-transfer-format">
					<Switch>
						<Match when={formats().length == 0}>
							<span style="color:var(--fg3)">No formats available. Try installing plugins.</span>
						</Match>
						<Match when={formats().length > 0}>
							<InlineSelect
								options={formats().map((format) => {
									return {
										value: format.id,
										contents: <div class="cont">{format.name}</div>,
										color: format.color,
									};
								})}
								selected={selectedFormat()}
								onChange={setSelectedFormat}
								connected={false}
								columns={1}
							/>

						</Match>
					</Switch>
				</div>
			</Tip>
		</div>
		<div></div>
		<div></div>
		<div class="cont">
			<IconTextButton
				size="1rem"
				text="Cancel"
				onClick={props.onClose}
			/>
			<IconTextButton
				icon={Cycle}
				size="1rem"
				text="Migrate"
				onClick={async () => {
					if (selectedFormat() == undefined) {
						inputError("instance-transfer-format");
						return;
					} else {
						clearInputError("instance-transfer-format");
					}


					try {
						let count: number = await invoke("migrate_instances", {
							format: selectedFormat(),
						});
						successToast(`Migrated ${count} instances`);
						updateInstanceList();
						props.onClose();
					} catch (e) {
						errorToast("Failed to migrate: " + e);
						props.onClose();
					}
				}}
			/>
		</div>
	</div>;
}

export interface MigratePromptProps {
	visible: boolean;
	onClose: () => void;
}
