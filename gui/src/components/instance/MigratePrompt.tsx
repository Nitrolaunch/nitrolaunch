import { createResource, createSignal, Match, Show, Switch } from "solid-js";
import ModalBase from "../dialog/ModalBase";
import { invoke } from "@tauri-apps/api/core";
import "./TemplateDeletePrompt.css";
import InlineSelect from "../input/select/InlineSelect";
import IconTextButton from "../input/button/IconTextButton";
import { Box, Copy, Cycle, Link, Properties } from "../../icons";
import { errorToast, successToast } from "../dialog/Toasts";
import { clearInputError, inputError } from "../../errors";
import Icon from "../Icon";
import Tip from "../dialog/Tip";
import { InstanceTransferFormat } from "./InstanceTransferPrompt";
import { updateInstanceList } from "../../pages/instance/InstanceList";

export default function MigratePrompt(props: MigratePromptProps) {
	return (
		<ModalBase visible={props.visible} onClose={props.onClose} width="35rem">
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
	let [instanceMode, setInstanceMode] = createSignal<"all" | "select">("all");
	let [selectedInstances, setSelectedInstances] = createSignal<string[]>([]);
	let [method, setMethod] = createSignal<"link" | "copy">("link");

	// Info about the launcher and available instances grabbed from the plugin
	let [launcherInfo, __] = createResource(
		() => selectedFormat(),
		async () => {
			if (selectedFormat() == undefined) {
				return undefined;
			}

			try {
				return (await invoke("check_migration", {
					format: selectedFormat()!,
				})) as CheckMigrationResult;
			} catch (e) {
				errorToast("Failed to get launcher info: " + e);
			}
		}
	);

	return (
		<div class="cont col fullwidth" style="padding:2rem;box-sizing:border-box">
			<div class="cont bold">
				<Icon icon={Cycle} size="1rem" />
				Migrate from Launcher
			</div>
			<div></div>
			<div class="cont fields" style="width:100%">
				<div class="cont start label">
					<label>FORMAT</label>
				</div>
				<Tip fullwidth tip="The launcher to migrate from" side="top">
					<div class="fullwidth" id="instance-transfer-format">
						<Switch>
							<Match when={formats().length == 0}>
								<span style="color:var(--fg3)">
									No formats available. Try installing plugins.
								</span>
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
									onChange={(x) => {
										if (x != selectedFormat()) {
											setSelectedInstances([]);
										}
										setSelectedFormat(x);
									}}
									connected={false}
									columns={1}
								/>
							</Match>
						</Switch>
					</div>
				</Tip>
				<hr />
				<Switch>
					<Match
						when={launcherInfo() == undefined && selectedFormat() != undefined}
					>
						Launcher not found on your computer
					</Match>
					<Match when={launcherInfo() != undefined}>
						<div class="cont start label">
							<label>INSTANCES</label>
						</div>
						<Tip
							fullwidth
							tip="What instances do you want to install?"
							side="top"
						>
							<div class="fullwidth" id="instance-transfer-instances">
								<InlineSelect
									options={[
										{
											value: "all",
											contents: (
												<div class="cont">
													<Icon icon={Box} size="1rem" />
													All Instances
												</div>
											),
											color: "var(--template)",
										},
										{
											value: "select",
											contents: (
												<div class="cont">
													<Icon icon={Properties} size="1rem" />
													Select Instances
												</div>
											),
											color: "var(--instance)",
										},
									]}
									selected={instanceMode()}
									onChange={setInstanceMode}
									connected
									columns={2}
								/>
							</div>
						</Tip>
						<Show when={instanceMode() == "select"}>
							<div class="fullwidth" id="instance-transfer-instances">
								<InlineSelect
									options={launcherInfo()!.instances.map((instance) => {
										return {
											value: instance,
											contents: <div class="cont">{instance}</div>,
										};
									})}
									selected={selectedInstances()}
									onChangeMulti={setSelectedInstances}
									connected={false}
									columns={3}
								/>
							</div>
						</Show>
						<div class="cont start label">
							<label>METHOD</label>
						</div>
						<Tip
							fullwidth
							tip="How do you want to migrate the instances?"
							side="top"
						>
							<div class="fullwidth" id="instance-transfer-instances">
								<InlineSelect
									options={[
										{
											value: "link",
											contents: (
												<div class="cont">
													<Icon icon={Link} size="1rem" />
													Link
												</div>
											),
											color: "var(--template)",
											tip: "Use the existing instance files without copying.",
											tipSide: "bottom"
										},
										{
											value: "copy",
											contents: (
												<div class="cont">
													<Icon icon={Copy} size="1rem" />
													Copy
												</div>
											),
											color: "var(--instance)",
											tip: "Copy all the instance files",
											tipSide: "bottom"
										},
									]}
									selected={method()}
									onChange={setMethod}
									connected
									columns={2}
								/>
							</div>
						</Tip>
						<div></div>
						<div></div>
						<div></div>
					</Match>
				</Switch>
			</div>
			<div></div>
			<div></div>
			<div class="cont">
				<IconTextButton size="1rem" text="Cancel" onClick={props.onClose} />
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
							let instances =
								instanceMode() == "all" ? undefined : selectedInstances();
							let link = method() == "link";

							let count: number = await invoke("migrate_instances", {
								format: selectedFormat(),
								instances: instances,
								link: link,
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
		</div>
	);
}

export interface MigratePromptProps {
	visible: boolean;
	onClose: () => void;
}

interface CheckMigrationResult {
	instances: string[];
}
