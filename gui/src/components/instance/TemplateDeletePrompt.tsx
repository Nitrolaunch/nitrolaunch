import { createResource, For, Match, Suspense, Switch } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { InstanceOrTemplate } from "../../types";
import { beautifyString } from "../../utils";
import "./TemplateDeletePrompt.css";
import { Delete, Trash } from "../../icons";
import { errorToast, successToast } from "../dialog/Toasts";
import { useNavigate } from "@solidjs/router";
import Modal, { ModalButton } from "../dialog/Modal";

export default function TemplateDeletePrompt(props: TemplateDeletePromptProps) {
	let navigate = useNavigate();

	let [templateUsers, _] = createResource(
		() => props.template,
		async () => {
			if (props.template == undefined) {
				return undefined;
			}

			return (await invoke("get_template_users", {
				template: props.template,
			})) as [string, InstanceOrTemplate][];
		}
	);

	let canDelete = () => templateUsers() != undefined && templateUsers()!.length == 0;

	let buttons = () => {
		let out: ModalButton[] = [{
			text: "Cancel",
			icon: Delete,
			color: "var(--instance)",
			bgColor: "var(--instancebg)",
			onClick: props.onClose,
		}];

		if (canDelete()) {
			out.push(
				{
					text: "Delete template",
					icon: Trash,
					color: "var(--fg3)",
					onClick: async () => {
						try {
							await invoke("delete_template", {
								template: props.template,
							});
							successToast("Template deleted");
							props.onClose();
							navigate("/");
						} catch (e) {
							errorToast("Failed to delete template: " + e);
							props.onClose();
						}
					}
				}
			);
		}

		return out;
	}

	return (
		<Modal
			visible={props.visible}
			onClose={props.onClose}
			title={`Delete Template '${props.template}'`}
			titleIcon={Trash}
			buttons={buttons()}
		>
			<Suspense>
				<Switch>
					<Match
						when={!canDelete()}
					>
						<h3>
							Cannot delete this template as there are instances or templates
							using it
						</h3>
						<For each={templateUsers()!}>
							{(item) => (
								<div
									class="template-delete-prompt-entry"
									onclick={() => {
										props.onClose();
										navigate(`/${item[1]}_config/${item[0]}`);
									}}
								>
									<div
										class="cont template-delete-prompt-entry-type"
										style={`color:var(--${item[1]})`}
									>
										{beautifyString(item[1])}
									</div>
									<div class="cont start template-delete-prompt-entry-id">
										{item[0]}
									</div>
								</div>
							)}
						</For>
					</Match>
					<Match
						when={canDelete()}
					>
						<h3>Are you sure you want to delete this template?</h3>
					</Match>
				</Switch>
			</Suspense>
		</Modal>
	);
}

export interface TemplateDeletePromptProps {
	template?: string;
	visible: boolean;
	onClose: () => void;
}
