import { invoke } from "@tauri-apps/api/core";
import { Event, listen } from "@tauri-apps/api/event";
import {
	createMemo,
	createResource,
	createSignal,
	For,
	Match,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import "./InstanceConsole.css";
import InlineSelect from "../input/select/InlineSelect";
import SearchBar from "../input/text/SearchBar";
import Icon from "../Icon";
import { AngleDown, AngleRight, Error, Info, Text, Warning } from "../../icons";
import { errorToast } from "../dialog/Toasts";

export default function InstanceConsole(props: InstanceConsoleProps) {
	let outputElem!: HTMLDivElement;

	let [filter, setFilter] = createSignal("all");
	let [search, setSearch] = createSignal("");

	let [input, setInput] = createSignal("");

	let [output, outputMethods] = createResource(
		() => props.instanceId,
		async () => {
			let output = (await invoke("get_instance_output", {
				instanceId: props.instanceId,
			})) as string | undefined;

			if (output == undefined) {
				return undefined;
			}

			// Format the output into lines
			let lines = output.split("\n");

			scrollToBottom();

			return lines;
		}
	);

	// Listener for when the output updates
	let [unlisten, _] = createResource(async () => {
		let unlisten = await listen(
			"update_instance_stdio",
			(event: Event<string>) => {
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

	function scrollToBottom() {
		if (outputElem != undefined) {
			outputElem.scrollTop = outputElem.scrollHeight;
		}
	}

	return (
		<div class="cont col instance-console">
			<div class="cont split fullwidth instance-console-header">
				<InlineSelect
					options={[
						{
							value: "all",
							contents: (
								<div class="cont">
									<Icon icon={Text} size="1rem" />
									ALL
								</div>
							),
							tip: "Show all messages",
						},
						{
							value: "error",
							contents: (
								<div class="cont">
									<Icon icon={Error} size="1rem" />
									ERRORS
								</div>
							),
							color: "var(--error)",
							tip: "Show only errors",
						},
						{
							value: "warning",
							contents: (
								<div class="cont">
									<Icon icon={Warning} size="1rem" />
									WARNINGS
								</div>
							),
							color: "var(--warning)",
							tip: "Show only warnings",
						},
						{
							value: "info",
							contents: (
								<div class="cont">
									<Icon icon={Info} size="1rem" />
									INFO
								</div>
							),
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
						<div class="cont col instance-console-text" ref={outputElem}>
							<For each={output()!}>
								{(line) => {
									let cls = line.includes("INFO")
										? "info"
										: line.includes("WARN")
										? "warning"
										: line.includes("ERROR")
										? "error"
										: "";

									let isVisible = createMemo((input) => {
										let filter2 = filter();
										let search2 = search();

										if (input == undefined) {
											return true;
										}

										if (filter2 != "all" && filter2 != cls) {
											return false;
										}

										if (
											search2 != undefined &&
											search2.length > 0 &&
											!line
												.toLocaleLowerCase()
												.includes(search2.toLocaleLowerCase())
										) {
											return false;
										}

										return true;
									});

									return (
										<span
											class={`console-line ${cls}`}
											style={`${isVisible() ? "" : "display:none"}`}
										>
											{line}
										</span>
									);
								}}
							</For>
						</div>
						<div
							class="cont shadow bubble-hover instance-console-scroll"
							onclick={() => {
								scrollToBottom();
							}}
						>
							<Icon icon={AngleDown} size="1.5rem" />
						</div>
						<Show when={props.isServer}>
							<div class="fullwidth instance-console-input">
								<form
									onsubmit={async (e) => {
										e.preventDefault();

										try {
											await invoke("write_instance_input", {
												instanceId: props.instanceId,
												input: input() + "\n",
											});
											setInput("");
										} catch (e) {
											errorToast("Failed to send: " + e);
										}
									}}
								>
									<input
										class="fullwidth"
										value={input()}
										onchange={(e) => setInput(e.target.value)}
									/>
								</form>
								<div class="cont instance-console-input-prompt">
									<Icon icon={AngleRight} size="1rem" />
								</div>
							</div>
						</Show>
					</Match>
					<Match when={output.error != undefined}>
						Failed to load: {output.error}
					</Match>
					<Match when={output.loading}>Loading...</Match>
					<Match when={!output.loading}>No logs found</Match>
				</Switch>
			</div>
		</div>
	);
}

export interface InstanceConsoleProps {
	instanceId: string;
	isServer: boolean;
}
