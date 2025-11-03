import { createResource, createSignal, Match, Show, Switch } from "solid-js";
import { invoke } from "@tauri-apps/api";
import "./TemplateDeletePrompt.css";
import InlineSelect from "../input/select/InlineSelect";
import { Delete, Download, Popout } from "../../icons";
import { errorToast, successToast } from "../dialog/Toasts";
import { clearInputError, inputError } from "../../errors";
import { open, save } from "@tauri-apps/api/dialog";
import Tip from "../dialog/Tip";
import { sanitizeInstanceId } from "../../pages/instance/InstanceConfig";
import { updateInstanceList } from "../../pages/instance/InstanceList";
import Modal from "../dialog/Modal";

export default function InstanceTransferPrompt(
	props: InstanceTransferPromptProps
) {
	let isImporting = () => props.exportedInstance == undefined;

	let [formats, _] = createResource(
		() => props.visible,
		async () => {
			let formats = (await invoke(
				"get_instance_transfer_formats"
			)) as InstanceTransferFormat[];

			// Filter out formats that don't support whether we are importing / exporting

			formats = formats.filter((format) => {
				if (isImporting() && format.import == undefined) {
					return false;
				}
				if (!isImporting() && format.export == undefined) {
					return false;
				}

				return true;
			});

			return formats;
		},
		{ initialValue: [] }
	);

	let [selectedFormat, setSelectedFormat] = createSignal<string | undefined>();
	let [newInstanceId, setNewInstanceId] = createSignal("");

	return (
		<Modal
			visible={props.visible}
			onClose={props.onClose}
			// title={
			// 	<Switch>
			// 		<Match when={isImporting()}>
			// 			<Icon icon={Download} size="1.2rem" />
			// 			Import Instance
			// 		</Match>
			// 		<Match when={!isImporting()}>
			// 			<Icon icon={Popout} size="1.2rem" />
			// 			Export Instance
			// 		</Match>
			// 	</Switch>
			// }
			title={isImporting() ? "Import Instance" : "Export Instance"}
			titleIcon={isImporting() ? Download : Popout}
			buttons={[
				{
					text: "Cancel",
					icon: Delete,
					onClick: props.onClose,
				},
				isImporting() ?
					{
						text: "Import",
						icon: Download,
						onClick: async () => {
							if (selectedFormat() == undefined) {
								inputError("instance-transfer-format");
								return;
							} else {
								clearInputError("instance-transfer-format");
							}

							if (newInstanceId().length == 0) {
								inputError("instance-transfer-id");
								return;
							} else {
								clearInputError("instance-transfer-id");
							}

							try {
								let filePath = await open();

								if (filePath == null) {
									return;
								}

								try {
									await invoke("import_instance", {
										format: selectedFormat(),
										id: newInstanceId(),
										path: filePath,
									});
									successToast("Instance imported");
									updateInstanceList();
									props.onClose();
								} catch (e) {
									errorToast("Failed to import: " + e);
									props.onClose();
								}
							} catch (e) {
								errorToast("Failed to select file: " + e);
							}
						}
					}
					:
					{
						text: "Export",
						icon: Popout,
						onClick: async () => {
							if (selectedFormat() == undefined) {
								inputError("instance-transfer-format");
								return;
							} else {
								clearInputError("instance-transfer-format");
							}

							try {
								let filePath = await save();

								if (filePath == null) {
									return;
								}

								try {
									await invoke("export_instance", {
										format: selectedFormat(),
										id: props.exportedInstance,
										path: filePath,
									});
									successToast("Instance exported");
									props.onClose();
								} catch (e) {
									errorToast("Failed to export: " + e);
									props.onClose();
								}
							} catch (e) {
								errorToast("Failed to select file: " + e);
							}
						}
					}
			]
			}
		>
			<div class="cont bold">

			</div>
			<div></div>
			<div class="cont fields" style="width:100%">
				<div class="cont start label">
					<label>FORMAT</label>
				</div>
				<Tip
					fullwidth
					tip="The format to use for the instance. Add new formats with plugins."
				>
					<div class="fullwidth" id="instance-transfer-format">
						<Switch>
							<Match when={formats().length == 0}>
								<span style="color:var(--fg3)">No formats available. Try installing a plugin.</span>
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
				<Show when={isImporting()}>
					<div class="cont start label">
						<label for="id">ID</label>
					</div>
					<Tip tip="A unique ID for the new instance" fullwidth>
						<input
							type="text"
							id="instance-transfer-id"
							name="id"
							onChange={(e) => {
								e.target.value = sanitizeInstanceId(e.target.value);
								setNewInstanceId(e.target.value);
							}}
							onKeyUp={(e: any) => {
								e.target.value = sanitizeInstanceId(e.target.value);
							}}
						></input>
					</Tip>
				</Show>
			</div>
		</Modal>
	);
}

export interface InstanceTransferPromptProps {
	// The ID of the exported instance. If undefined, we are importing an instance.
	exportedInstance?: string;
	visible: boolean;
	onClose: () => void;
}

export interface InstanceTransferFormat {
	id: string;
	name: string;
	color: string | undefined;
	import: any | undefined;
	export: any | undefined;
	migrate: any | undefined;
}
