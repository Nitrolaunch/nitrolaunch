import { createSignal, For, JSX, Show } from "solid-js";
import "./InlineSelect.css";
import Tip, { TipSide } from "../../dialog/Tip";
import Icon from "../../Icon";
import { Check } from "../../../icons";

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
			class={`${connected ? "shadow" : ""} inline-select ${
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
					checkboxes={props.checkboxes == true}
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
						checkboxes={props.checkboxes == true}
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
	checkboxes?: boolean;
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
			if (props.option.selectedBgColor != undefined) {
				return props.option.selectedBgColor;
			} else if (props.solidSelect) {
				return color;
			} else {
				return "var(--bg0)";
			}
		} else {
			return "var(--bg2)";
		}
	};
	let borderColor = () =>
		isSelected() ? color : isHovered() ? "var(--bg4)" : "var(--bg3)";

	let contents = (
		<div
			class={`cont inline-select-option ${
				props.connected ? "connected" : "disconnected shadow bubble-hover"
			} ${props.class == undefined ? "" : props.class} ${
				isSelected() ? "selected" : ""
			} ${props.isLast ? "last" : "not-last"} ${
				props.isFirst ? "" : "not-first"
			}`}
			style={`border-color:${borderColor()};color:${textColor()};background-color:${backgroundColor()}`}
			onclick={() => props.onSelect(props.option.value)}
			onmouseenter={() => setIsHovered(true)}
			onmouseleave={() => setIsHovered(false)}
		>
			<Show when={props.checkboxes}>
				<div
					class="cont inline-select-checkbox"
					style={
						isSelected()
							? `background-color:${borderColor()}`
							: `border-color:${borderColor()}`
					}
				>
					<Show when={isSelected()}>
						<Icon icon={Check} size="0.75rem" />
					</Show>
				</div>
			</Show>
			{props.option.contents}
		</div>
	);

	if (props.option.tip == undefined) {
		return contents;
	} else {
		return (
			<Tip
				tip={<div style="color:var(--fg)">{props.option.tip}</div>}
				side={props.option.tipSide == undefined ? "top" : props.option.tipSide}
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
	checkboxes: boolean;
	onSelect: (option: string | undefined) => void;
}

export interface Option {
	value: string | undefined;
	contents: JSX.Element;
	color?: string;
	selectedTextColor?: string;
	selectedBgColor?: string;
	tip?: JSX.Element;
	tipSide?: TipSide;
}
