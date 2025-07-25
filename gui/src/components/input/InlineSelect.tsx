import { createSignal, For, JSX, Show } from "solid-js";
import "./InlineSelect.css";
import Tip from "../dialog/Tip";

export default function InlineSelect(props: InlineSelectProps) {
	let columns = () => (props.columns == undefined ? 3 : props.columns);
	let connected = props.connected == undefined ? true : props.connected;
	let grid = props.grid == undefined ? true : props.grid;
	let solidSelect = props.solidSelect == undefined ? false : props.solidSelect;

	let selectFunction = (value: string | undefined) => {
		if (props.onChange != undefined) {
			props.onChange(value);
		}
		if (props.onChangeMulti != undefined) {
			if (Array.isArray(props.selected)) {
				let array = props.selected.includes(value!)
					? props.selected.filter((x) => x != value)
					: props.selected.concat(value!);
				props.onChangeMulti(array);
			} else {
				if (value == undefined) {
					props.onChangeMulti(undefined);
				} else {
					props.onChangeMulti([value]);
				}
			}
		}
	};

	return (
		<div
			class={`${connected ? "input-shadow" : ""} inline-select ${
				connected ? "connected" : "disconnected"
			}`}
			style={`display:${
				grid ? "grid" : "flex"
			};grid-template-columns:repeat(${columns()}, minmax(0, 1fr))`}
		>
			<Show when={props.allowEmpty == undefined ? false : props.allowEmpty}>
				<InlineSelectOption
					option={{
						value: undefined,
						contents: "None",
					}}
					connected={connected}
					onSelect={selectFunction}
					selected={props.selected}
					isLast={
						props.options.length == 0 ||
						props.selected == props.options[0].value
					}
					isFirst={true}
					class={props.optionClass}
					solidSelect={solidSelect}
				/>
			</Show>
			<For each={props.options}>
				{(option, index) => (
					<InlineSelectOption
						option={option}
						connected={connected}
						onSelect={selectFunction}
						selected={props.selected}
						isLast={index() == props.options.length - 1}
						isFirst={index() == 0 && !props.allowEmpty}
						class={props.optionClass}
						solidSelect={solidSelect}
					/>
				)}
			</For>
		</div>
	);
}

export interface InlineSelectProps {
	options: Option[];
	selected?: string | string[];
	onChange?: (option: string | undefined) => void;
	onChangeMulti?: (options: string[] | undefined) => void;
	columns?: number;
	allowEmpty?: boolean;
	connected?: boolean;
	optionClass?: string;
	grid?: boolean;
	solidSelect?: boolean;
}

function InlineSelectOption(props: OptionProps) {
	let [isHovered, setIsHovered] = createSignal(false);

	let isSelected = () => {
		return Array.isArray(props.selected) && props.selected != undefined
			? props.selected.includes(props.option.value!)
			: props.selected == props.option.value;
	};
	let color =
		props.option.color == undefined ? "var(--fg2)" : props.option.color;

	let textColor = () => {
		if (isSelected()) {
			if (props.option.selectedTextColor == undefined) {
				if (props.solidSelect) {
					return "black";
				} else {
					return color;
				}
			} else {
				return props.option.selectedTextColor;
			}
		} else {
			return "var(--fg)";
		}
	};

	let backgroundColor = () => {
		if (isSelected()) {
			if (props.solidSelect) {
				return color;
			} else {
				return "var(--bg)";
			}
		} else {
			return "var(--bg2)";
		}
	};
	let borderColor = () =>
		`border-color:${
			isSelected() ? color : isHovered() ? "var(--bg4)" : "var(--bg3)"
		}`;

	let contents = (
		<div
			class={`cont inline-select-option ${
				props.connected ? "connected" : "disconnected input-shadow"
			} ${props.class == undefined ? "" : props.class} ${
				isSelected() ? "selected" : ""
			} ${props.isLast ? "last" : "not-last"} ${
				props.isFirst ? "" : "not-first"
			}`}
			style={`${borderColor()};color:${textColor()};background-color:${backgroundColor()}`}
			onclick={() => props.onSelect(props.option.value)}
			onmouseenter={() => setIsHovered(true)}
			onmouseleave={() => setIsHovered(false)}
		>
			{props.option.contents}
		</div>
	);

	if (props.option.tip == undefined) {
		return contents;
	} else {
		return (
			<Tip
				tip={<div style="color:var(--fg)">{props.option.tip}</div>}
				side="top"
			>
				{contents}
			</Tip>
		);
	}
}

interface OptionProps {
	option: Option;
	selected?: string | string[];
	connected: boolean;
	solidSelect: boolean;
	class?: string;
	isFirst: boolean;
	isLast: boolean;
	onSelect: (option: string | undefined) => void;
}

export interface Option {
	value: string | undefined;
	contents: JSX.Element;
	color?: string;
	selectedTextColor?: string;
	tip?: JSX.Element;
}
