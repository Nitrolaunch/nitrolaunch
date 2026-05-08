import { invoke } from "@tauri-apps/api/core";
import Modal from "../dialog/Modal";
import { errorToast, successToast } from "../dialog/Toasts";
import { updateInstanceList } from "../../pages/instance/InstanceList";
import { useNavigate } from "@solidjs/router";
import { Copy, Delete, Download, Trash, Upload } from "../../icons";
import { createEffect, createSignal, Match, Switch } from "solid-js";
import IdInput from "../input/text/IdInput";
import Tip from "../dialog/Tip";

export default function InstanceOperationPrompt(props: InstanceOperationPromptProps) {
	let navigate = useNavigate();

	let [newId, setNewId] = createSignal("");
	createEffect(() => {
		if (props.visible) {
			setNewId("");
		}
	})

	let title = () => {
		if (props.operation == "delete") {
			return "Delete Instance";
		} else if (props.operation == "consolidate") {
			return "Consolidate instance";
		} else if (props.operation == "duplicate") {
			return "Duplicate instance";
		} else if (props.operation == "extract") {
			return "Extract instance";
		} else {
			return "";
		}
	};

	let icon = () => {
		if (props.operation == "delete") {
			return Trash;
		} else if (props.operation == "consolidate") {
			return Download;
		} else if (props.operation == "duplicate") {
			return Copy;
		} else if (props.operation == "extract") {
			return Upload;
		} else {
			return Trash;
		}
	};

	let confirmColor = () => {
		if (props.operation == "delete") {
			return "var(--fg3)";
		} else {
			return "var(--fg)";
		}
	}

	let deleteInstance = async () => {
		try {
			await invoke("delete_instance", { instance: props.instanceId });
			successToast("Instance deleted");
			props.onClose();
			updateInstanceList();
			navigate("/");
		} catch (e) {
			errorToast("Failed to delete instance: " + e);
		}
	};

	let consolidateInstance = async () => {
		try {
			await invoke("consolidate_instance", { instance: props.instanceId });
			successToast("Instance consolidated");
			props.onClose();
		} catch (e) {
			errorToast("Failed to consolidate instance: " + e);
		}
	};

	let duplicateInstance = async () => {
		try {
			await invoke("duplicate_instance", { instance: props.instanceId, newId: newId() });
			successToast("Instance duplicated");
			updateInstanceList();
			props.onClose();
		} catch (e) {
			errorToast("Failed to duplicate instance: " + e);
		}
	};

	let extractInstance = async () => {
		try {
			await invoke("extract_instance", { instance: props.instanceId, newId: newId() });
			successToast("Template extracted from instance");
			updateInstanceList();
			props.onClose();
		} catch (e) {
			errorToast("Failed to extract template from instance: " + e);
		}
	};

	return <Modal
		visible={props.visible}
		onClose={props.onClose}
		title={title()}
		titleIcon={icon()}
		buttons={[
			{
				text: "Cancel",
				icon: Delete,
				color: "var(--confirm)",
				bgColor: "var(--confirmbg)",
				onClick: props.onClose,
			},
			{
				text: title(),
				icon: icon(),
				color: confirmColor(),
				onClick: () => {
					if (props.operation == "delete") {
						deleteInstance();
					} else if (props.operation == "consolidate") {
						consolidateInstance();
					} else if (props.operation == "duplicate") {
						duplicateInstance();
					} else if (props.operation == "extract") {
						extractInstance();
					}
				},
			},
		]}
	>
		<div class="cont col fields">
			<Switch>
				<Match when={props.operation == "delete"}>
					<h3>Are you sure you want to delete this instance?</h3>
					<div class="cont bold" style="font-size:0.9rem;color:var(--fg2)">
						This will delete ALL of your worlds and data for the instance!
					</div>
				</Match>
				<Match when={props.operation == "consolidate"}>
					<span>Consolidate the templates that this instance inherits from, unlinking from them and combining all the config into this instance</span>
					<span>Will leave the templates untouched</span>
				</Match>
				<Match when={props.operation == "duplicate"}>
					<span>Create another copy of this instance</span>
					<label class="label">
						ID
					</label>
					<Tip
						tip="ID for the new instance copy"
						fullwidth
						side="top"
					>
						<IdInput value={newId()} onChange={setNewId} />
					</Tip>
				</Match>
				<Match when={props.operation == "extract"}>
					<span>Extract most of the config for this instance into a new template, so you can share it with other instances</span>
					<span>Removes that config from this instance, and inherits from the new template</span>
					<label class="label">
						ID
					</label>
					<Tip
						tip="ID for the new template"
						fullwidth
						side="top"
					>
						<IdInput value={newId()} onChange={setNewId} />
					</Tip>
				</Match>
			</Switch>
		</div>
	</Modal>;
}

export interface InstanceOperationPromptProps {
	instanceId: string;
	operation: InstanceOperation;
	visible: boolean;
	onClose: () => void;
}

export type InstanceOperation = "delete" | "consolidate" | "duplicate" | "extract";
