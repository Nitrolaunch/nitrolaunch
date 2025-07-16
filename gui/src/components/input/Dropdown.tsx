import { createMemo, createSignal, Index, JSX, Show } from "solid-js";
import "./Dropdown.css";
import Tip from "../dialog/Tip";
import { canonicalizeListOrSingle } from "../../utils/values";
import Icon from "../Icon";
import { AngleDown, AngleRight } from "../../icons";

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

	let zIndex = props.zIndex == undefined ? "" : `z-index:${props.zIndex}`;

	let headerContents = () => {
		if (props.onChangeMulti == undefined) {
			let option = props.options.find((x) => x.value == props.selected);
			return option == undefined ? "None" : option.contents;
		} else {
			return `${canonicalizeListOrSingle(props.selected).length} selected`;
		}
	};

	return (
		<div class="dropdown-container" onmouseleave={() => setIsOpen(false)}>
			<div
				class={`cont input-shadow dropdown-header ${isOpen() ? "open" : ""}`}
				onclick={() => setIsOpen(!isOpen())}
			>
				{headerContents()}
				<div class="cont dropdown-arrow">
					<Show
						when={isOpen()}
						fallback={<Icon icon={AngleRight} size="1rem" />}
					>
						<Icon icon={AngleDown} size="1rem" />
					</Show>
				</div>
			</div>
			<div
				class="dropdown-options"
				style={`${
					!isOpen() ? "max-height:0px;border-width:0px" : ""
				};${zIndex}`}
			>
				<Show when={props.allowEmpty == undefined ? false : props.allowEmpty}>
					<DropdownOption
						option={{
							value: undefined,
							contents: "None",
						}}
						onSelect={selectFunction}
						isSelected={props.selected == undefined}
						isLast={props.options.length == 0}
						class={props.optionClass}
					/>
				</Show>
				<Index each={props.options}>
					{(option, index) => (
						<DropdownOption
							option={option()}
							onSelect={selectFunction}
							isSelected={createMemo(() =>
								props.selected != undefined && Array.isArray(props.selected)
									? props.selected.includes(option().value!)
									: props.selected == option().value
							)()}
							isLast={index == props.options.length - 1}
							class={props.optionClass}
						/>
					)}
				</Index>
			</div>
		</div>
	);
}

export interface DropdownProps {
	options: Option[];
	selected?: string | string[];
	onChange?: (option: string | undefined) => void;
	onChangeMulti?: (options: string[] | undefined) => void;
	allowEmpty?: boolean;
	optionClass?: string;
	zIndex?: string;
}

function DropdownOption(props: OptionProps) {
	let [isHovered, setIsHovered] = createSignal(false);

	let color =
		props.option.color == undefined ? "var(--fg2)" : props.option.color;

	let textColor = () => {
		if (props.isSelected) {
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
		if (props.isSelected || isHovered()) {
			return "var(--bg0)";
		} else {
			return "var(--bg)";
		}
	};

	let contents = (
		<div
			class={`cont dropdown-option ${
				props.class == undefined ? "" : props.class
			} ${props.isSelected ? "selected" : ""} ${
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
	isSelected: boolean;
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
