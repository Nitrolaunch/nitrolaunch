import { invoke } from "@tauri-apps/api/core";
import {
	createResource,
	createSignal,
} from "solid-js";
import { errorToast } from "./dialog/Toasts";
import Console from "./launch/Console";

export default function ApplicationLog() {
	let [selectedLog, setSelectedLog] = createSignal<string | undefined>();

	let [output, outputMethods] = createResource(
		() => selectedLog(),
		async (log) => {
			try {
				return (await invoke("get_log", {
					log: log,
				})) as string;
			} catch (e) {
				console.error(e);
			}
		},
	);

	let [availableLogs, __] = createResource(
		async () => {
			try {
				return (await invoke("get_logs")) as string[];
			} catch (e) {
				errorToast("Failed to fetch logs: " + e);
				return [];
			}
		},
		{ initialValue: [] },
	);

	return <Console
		loadState={output.state}
		output={output()}
		fetchOutput={outputMethods.refetch}
		sendMessage={undefined}
		availableLogs={availableLogs()}
		selectedLog={selectedLog()}
		setSelectedLog={setSelectedLog}
	/>;
}
