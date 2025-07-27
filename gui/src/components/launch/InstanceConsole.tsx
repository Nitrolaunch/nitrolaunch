import { invoke } from "@tauri-apps/api";
import { Event, listen } from "@tauri-apps/api/event";
import { createResource, Match, onCleanup, Switch } from "solid-js";
import "./InstanceConsole.css";

export default function InstanceConsole(props: InstanceConsoleProps) {
	let outputElem!: HTMLDivElement;

	let [output, outputMethods] = createResource(async () => {
		let output = (await invoke("get_instance_output", {
			instanceId: props.instanceId,
		})) as string | undefined;

		if (output != undefined) {
			output = output.replace("/INFO", '<div class="bold">/INFO</div>');
		}

		if (outputElem != undefined) {
			outputElem.scrollTop = outputElem.scrollHeight;
		}

		return output;
	});

	// Listener for when the output updates
	let [unlisten, _] = createResource(async () => {
		let unlisten = await listen(
			"update_instance_stdio",
			(event: Event<string>) => {
				console.log(event);
				if (event.payload == props.instanceId) {
					outputMethods.refetch();
				}
			}
		);

		return unlisten;
	});

	onCleanup(() => {
		if (unlisten() != undefined) {
			unlisten();
		}
	});

	return (
		<div class="cont instance-console">
			<div class="cont col instance-console-output">
				<Switch>
					<Match when={output() != undefined}>
						<div class="instance-console-text" ref={outputElem}>
							{output()}
						</div>
					</Match>
					<Match when={output.error != undefined}>
						Failed to load: {output.error}
					</Match>
					<Match when={output.loading}>Loading...</Match>
					<Match when={!output.loading}>Instance not Running</Match>
				</Switch>
			</div>
		</div>
	);
}

export interface InstanceConsoleProps {
	instanceId: string;
}
