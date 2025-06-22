import { createSignal, For, JSX, Show } from "solid-js";
import "./Dropdown.css";
import Tip from "../dialog/Tip";
import { canonicalizeListOrSingle } from "../../utils/values";
import Icon from "../Icon";
import { AngleDown, AngleRight } from "../../icons";
import DisplayShow from "../utility/DisplayShow";

export default function Dropdown(props: DropdownProps) {
	let [isOpen, setIsOpen] = createSignal(false);

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
		<div class="dropdown-container" onmouseleave={() => setIsOpen(false)}>
			<div
				class={`cont input-shadow dropdown-header ${isOpen() ? "open" : ""}`}
				onclick={() => setIsOpen(!isOpen())}
			>
				{props.onChangeMulti == undefined
					? props.selected
					: `${canonicalizeListOrSingle(props.selected).length} selected`}
				<div class="cont dropdown-arrow">
					<Show
						when={isOpen()}
						fallback={<Icon icon={AngleRight} size="1rem" />}
					>
						<Icon icon={AngleDown} size="1rem" />
					</Show>
				</div>
			</div>
			<DisplayShow when={isOpen()}>
				<div
					class="dropdown-options"
					style={`${!isOpen() ? "max-height:0px" : ""}`}
				>
					<Show when={props.allowEmpty == undefined ? false : props.allowEmpty}>
						<DropdownOption
							option={{
								value: undefined,
								contents: "None",
							}}
							onSelect={selectFunction}
							selected={props.selected}
							isLast={props.selected == props.options[0].value}
							class={props.optionClass}
						/>
					</Show>
					<For each={props.options}>
						{(option, index) => (
							<DropdownOption
								option={option}
								onSelect={selectFunction}
								selected={props.selected}
								isLast={index() == props.options.length - 1}
								class={props.optionClass}
							/>
						)}
					</For>
				</div>
			</DisplayShow>
		</div>
	);
}

export interface DropdownProps {
	options: Option[];
	selected?: string | string[];
	onChange?: (option: string | undefined) => void;
	onChangeMulti?: (options: string[] | undefined) => void;
	allowEmpty?: boolean;
	connected?: boolean;
	optionClass?: string;
	grid?: boolean;
	solidSelect?: boolean;
}

function DropdownOption(props: OptionProps) {
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
				return color;
			} else {
				return props.option.selectedTextColor;
			}
		} else {
			return "var(--fg)";
		}
	};

	let backgroundColor = () => {
		if (isSelected() || isHovered()) {
			return "var(--bg0)";
		} else {
			return "var(--bg)";
		}
	};

	let contents = (
		<div
			class={`cont dropdown-option ${
				props.class == undefined ? "" : props.class
			} ${isSelected() ? "selected" : ""} ${
				props.isLast ? "last" : "not-last"
			}`}
			style={`color:${textColor()};background-color:${backgroundColor()}`}
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
				side="right"
			>
				{contents}
			</Tip>
		);
	}
}

interface OptionProps {
	option: Option;
	selected?: string | string[];
	class?: string;
	isLast?: boolean;
	onSelect: (option: string | undefined) => void;
}

export interface Option {
	value: string | undefined;
	contents: JSX.Element;
	color?: string;
	selectedTextColor?: string;
	tip?: JSX.Element;
}
