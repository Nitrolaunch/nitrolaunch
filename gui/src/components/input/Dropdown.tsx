import {
	createEffect,
	createMemo,
	createSignal,
	For,
	JSX,
	Match,
	Show,
	Switch,
} from "solid-js";
import "./Dropdown.css";
import Tip from "../dialog/Tip";
import { canonicalizeListOrSingle, undefinedEmpty } from "../../utils/values";
import Icon from "../Icon";
import { AngleDown, AngleRight } from "../../icons";

export default function Dropdown(props: DropdownProps) {
	let [isOpen, setIsOpen] = createSignal(props.startOpen == true);

	let selectFunction = (value: string | undefined) => {
		if (props.onChange != undefined) {
			props.onChange(value);
			setIsOpen(false);
		}
		if (props.onChangeMulti != undefined) {
			console.log("select");
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

	let [search, setSearch] = createSignal<string | undefined>();

	let searchElement!: HTMLInputElement;

	createEffect(() => {
		if (isOpen()) {
			searchElement.focus();
		}
	});

	return (
		<div class="dropdown-container">
			<div
				class={`cont input-shadow dropdown-header ${isOpen() ? "open" : ""}`}
				onclick={() => setIsOpen(!isOpen())}
			>
				<Switch>
					<Match when={!isOpen()}>{headerContents()}</Match>
					<Match when={isOpen()}>
						<input
							type="text"
							class="dropdown-search"
							style="padding-left:0.5rem"
							onclick={(e) => {
								e.preventDefault();
								e.stopPropagation();
							}}
							onkeyup={(e: any) => {
								let search = undefinedEmpty(e.target.value);
								setSearch(search);
								if (props.customSearchFunction != undefined) {
									props.customSearchFunction(search);
								}
							}}
							onkeydown={(e: any) => {
								// Unfocus on escape
								if (e.keyCode == 27) {
									e.target.blur();
								}
							}}
							ref={searchElement}
						/>
					</Match>
				</Switch>
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
				onmouseleave={() => setIsOpen(false)}
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
				<For each={props.options}>
					{(option, index) => {
						let isVisible = createMemo(() => {
							return (
								search() == undefined ||
								props.customSearchFunction != undefined ||
								(option.value != undefined && option.value!.includes(search()!))
							);
						});

						return (
							<Show when={isVisible()}>
								<DropdownOption
									option={option}
									onSelect={selectFunction}
									isSelected={createMemo(() =>
										props.selected != undefined && Array.isArray(props.selected)
											? props.selected.includes(option.value!)
											: props.selected == option.value
									)()}
									isLast={index() == props.options.length - 1}
									class={props.optionClass}
								/>
							</Show>
						);
					}}
				</For>
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
	customSearchFunction?: (search: string | undefined) => void;
	startOpen?: boolean;
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
			return "var(--bg)";
		} else {
			return "var(--bg2)";
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
			onclick={() => {
				if (
					props.option.isSelectable == undefined ||
					props.option.isSelectable == true
				) {
					props.onSelect(props.option.value);
				}
			}}
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
	isSelectable?: boolean;
}
