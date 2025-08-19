import { invoke } from "@tauri-apps/api";
import { Event, listen } from "@tauri-apps/api/event";
import {
	createResource,
	createSignal,
	Match,
	onCleanup,
	Switch,
} from "solid-js";
import "./InstanceConsole.css";
import InlineSelect from "../input/InlineSelect";
import SearchBar from "../input/SearchBar";

export default function InstanceConsole(props: InstanceConsoleProps) {
	let outputElem!: HTMLDivElement;

	let [filter, setFilter] = createSignal("all");
	let [search, setSearch] = createSignal("");

	let [output, outputMethods] = createResource(
		() => filter() + search(),
		async () => {
			let output = (await invoke("get_instance_output", {
				instanceId: props.instanceId,
			})) as string | undefined;

			if (output == undefined) {
				return undefined;
			}

			// Format the output into lines

			let lines = output.split("\n");

			let out = "";

			for (let line of lines) {
				let cls = line.includes("INFO")
					? "info"
					: line.includes("WARN")
					? "warning"
					: line.includes("ERROR")
					? "error"
					: "";

				if (filter() != "all" && filter() != cls) {
					continue;
				}

				if (
					search() != undefined &&
					search().length > 0 &&
					!line.toLocaleLowerCase().includes(search().toLocaleLowerCase())
				) {
					continue;
				}

				out += `<div class="console-line ${cls}">${line}</div>`;
			}

			if (outputElem != undefined) {
				outputElem.scrollTop = outputElem.scrollHeight;
			}

			return out;
		}
	);

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
		<div class="cont col instance-console">
			<div
				class="cont split fullwidth"
				style="padding: 1rem;padding-bottom:0;padding-top:0; box-sizing:border-box"
			>
				<InlineSelect
					options={[
						{
							value: "all",
							contents: "ALL",
							tip: "Show all messages",
						},
						{
							value: "error",
							contents: "ERRORS",
							color: "var(--error)",
							tip: "Show only errors",
						},
						{
							value: "warning",
							contents: "WARNINGS",
							color: "var(--warning)",
							tip: "Show only warnings",
						},
						{
							value: "info",
							contents: "INFO",
							color: "var(--fg3)",
							tip: "Show only info messages",
						},
					]}
					selected={filter()}
					onChange={setFilter}
					connected={false}
					columns={4}
					solidSelect
				/>
				<div class="cont end fullwidth">
					<SearchBar value={search()} method={setSearch} immediate />
				</div>
			</div>
			<div class="cont col instance-console-output">
				<Switch>
					<Match when={output() != undefined}>
						<div
							class="instance-console-text"
							ref={outputElem}
							innerHTML={output()}
						></div>
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
