import { invoke } from "@tauri-apps/api/core";
import { Event, listen } from "@tauri-apps/api/event";
import {
	createEffect,
	createResource,
	createSignal,
	onCleanup,
} from "solid-js";
import { errorToast } from "../dialog/Toasts";
import Console from "./Console";

export default function InstanceConsole(props: InstanceConsoleProps) {
	// Undefined for the current instance output
	let [selectedLog, setSelectedLog] = createSignal<string | undefined>();

	let [output, outputMethods] = createResource(
		() => props.instanceId,
		async () => {
			try {
				let text = "";
				if (selectedLog() == undefined) {
					let output = (await invoke("get_instance_output", {
						instanceId: props.instanceId,
					})) as string | undefined;

					if (output == undefined) {
						return undefined;
					}
					text = output;
				} else {
					text = (await invoke("get_instance_log", {
						instanceId: props.instanceId,
						logId: selectedLog(),
					})) as string;
				}

				return text;
			} catch (e) {
				console.error(e);
			}
		},
	);

	createEffect(() => {
		selectedLog();
		outputMethods.refetch();
	});

	// Listener for when the output updates
	let [unlisten, _] = createResource(async () => {
		let unlisten = await listen(
			"update_instance_stdio",
			(event: Event<string>) => {
				if (event.payload == props.instanceId && selectedLog() == undefined) {
					outputMethods.refetch();
				}
			},
		);

		return unlisten;
	});

	onCleanup(() => {
		if (unlisten() != undefined) {
			unlisten();
		}
	});

	let [availableLogs, __] = createResource(
		async () => {
			try {
				return (await invoke("get_instance_logs", {
					instanceId: props.instanceId,
				})) as string[];
			} catch (e) {
				errorToast("Failed to fetch instance logs: " + e);
				return [];
			}
		},
		{ initialValue: [] },
	);

	let sendMessage = async (message: string) => {
		try {
			await invoke("write_instance_input", {
				instanceId: props.instanceId,
				input: message,
			});
		} catch (e) {
			errorToast("Failed to send: " + e);
		}
	}

	return <Console
		loadState={output.state}
		output={output()}
		fetchOutput={outputMethods.refetch}
		sendMessage={props.isServer ? sendMessage : undefined}
		availableLogs={availableLogs()}
		selectedLog={selectedLog()}
		setSelectedLog={setSelectedLog}
	/>;
}

export interface InstanceConsoleProps {
	instanceId: string;
	isServer: boolean;
}
