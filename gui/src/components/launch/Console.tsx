import {
	createMemo,
	createSignal,
	For,
	Match,
	Show,
	Switch,
} from "solid-js";
import "./Console.css";
import InlineSelect from "../input/select/InlineSelect";
import SearchBar from "../input/text/SearchBar";
import Icon from "../Icon";
import { AngleDown, AngleRight, Error, Info, Text, Warning } from "../../icons";
import Dropdown, { Option } from "../input/select/Dropdown";

// Console for logs
export default function Console(props: ConsoleProps) {
	let outputElem!: HTMLDivElement;

	let [filter, setFilter] = createSignal("all");
	let [search, setSearch] = createSignal("");

	let [input, setInput] = createSignal("");

	let output = () => {
		if (props.output == undefined) {
			return undefined;
		} else {
			let lines = props.output.split("\n");

			return lines;
		}
	};

	function scrollToBottom() {
		if (outputElem != undefined) {
			outputElem.scrollTop = outputElem.scrollHeight;
		}
	}

	return (
		<div class="cont col console">
			<div class="cont fullwidth console-header">
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
				<div class="cont">
					<div class="cont" style="width:14rem">
						<Dropdown
							options={[
								{
									value: undefined,
									contents: "Current Output",
								} as Option,
							].concat(
								props.availableLogs.map((x) => {
									return {
										value: x,
										contents: x,
									};
								}),
							)}
							selected={props.selectedLog}
							onChange={props.setSelectedLog}
							zIndex="2"
						/>
					</div>
				</div>
				<div class="cont end fullwidth">
					<SearchBar value={search()} method={setSearch} immediate />
				</div>
			</div>
			<div class="cont col console-output">
				<Switch>
					<Match when={output() != undefined}>
						<div class="cont col console-text" ref={outputElem}>
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
							class="cont shadow bubble-hover console-scroll"
							onclick={() => {
								scrollToBottom();
							}}
						>
							<Icon icon={AngleDown} size="1.5rem" />
						</div>
						<Show when={props.sendMessage != undefined}>
							<div class="fullwidth console-input">
								<form
									onsubmit={async (e) => {
										e.preventDefault();
										props.sendMessage!(input() + "\n");
										setInput("");
									}}
								>
									<input
										class="fullwidth"
										value={input()}
										onchange={(e) => setInput(e.target.value)}
									/>
								</form>
								<div class="cont console-input-prompt">
									<Icon icon={AngleRight} size="1rem" />
								</div>
							</div>
						</Show>
					</Match>
					<Match when={props.loadState == "errored"}>
						Failed to load
					</Match>
					<Match when={props.loadState == "pending"}>Loading...</Match>
					<Match when={props.loadState == "ready"}>No log found</Match>
				</Switch>
			</div>
		</div>
	);
}

export interface ConsoleProps {
	loadState: "pending" | "ready" | "errored" | "unresolved" | "refreshing";
	fetchOutput: (log: string | undefined) => void;
	output?: string;
	selectedLog?: string;
	setSelectedLog: (log: string | undefined) => void;
	availableLogs: string[];
	sendMessage?: (message: string) => void;
}
