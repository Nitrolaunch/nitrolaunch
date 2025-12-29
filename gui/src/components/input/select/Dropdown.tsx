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
import Tip from "../../dialog/Tip";
import { canonicalizeListOrSingle, undefinedEmpty } from "../../../utils/values";
import Icon from "../../Icon";
import { AngleRight, Search } from "../../../icons";

export default function Dropdown(props: DropdownProps) {
	let [isOpen, setIsOpen] = createSignal(props.startOpen == true);

	// Hovering behavior
	let [isHeaderHovered, setIsHeaderHovered] = createSignal(isOpen());
	let [isGapHovered, setIsGapHovered] = createSignal(false);
	let [areOptionsHovered, setAreOptionsHovered] = createSignal(false);

	createEffect(() => {
		let anyPartHovered = () =>
			isHeaderHovered() || isGapHovered() || areOptionsHovered();
		if (!anyPartHovered()) {
			// We need to set a timeout since when we switch between elements there is an instant where nothing is hovered
			setTimeout(() => {
				if (!anyPartHovered()) {
					close();
				}
			}, 250);
		}
	});

	function close() {
		setIsOpen(false);
		if (props.onClose != undefined) {
			props.onClose!();
		}
	}

	let selectFunction = (value: string | undefined) => {
		if (props.onChange != undefined) {
			props.onChange(value);
			close();
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

	let allowEmpty = props.allowEmpty == undefined ? false : props.allowEmpty;

	let zIndex = props.zIndex == undefined ? "" : `z-index:${props.zIndex}`;

	let isSearchable =
		props.isSearchable == undefined ? true : props.isSearchable;

	let showArrow = props.showArrow == undefined ? true : props.showArrow;

	let headerContents = () => {
		if (props.previewText != undefined) {
			return props.previewText;
		} else if (props.onChangeMulti == undefined) {
			let option = props.options.find((x) => x.value == props.selected);
			return option == undefined ? "None" : option.contents;
		} else {
			return `${canonicalizeListOrSingle(props.selected).length} selected`;
		}
	};

	// The height of the opened dropdown options
	let openedHeight = () => {
		if (props.options.length < 10.5) {
			return `calc(${props.options.length} * var(--option-height) + 2 * var(--border))`;
		} else {
			return "calc(10.5 * var(--option-height) + 2 * var(--border))";
		}
	};

	let lastIndex = createMemo(() => props.options.length - 1);

	let [search, setSearch] = createSignal<string | undefined>();

	let searchElement!: HTMLInputElement;

	createEffect(() => {
		if (isOpen() && searchElement != undefined) {
			searchElement.focus();
			setSearch(undefined);
		}
	});

	return (
		<div class="dropdown-container">
			<div
				class={`dropdown-darkener ${isOpen() ? "open" : ""}`}
				style={`${zIndex}`}
				onmouseenter={() => setIsGapHovered(true)}
				onmouseleave={() => setIsGapHovered(false)}
			></div>
			<div
				class={`cont shadow ${isOpen() ? "" : "bubble-hover"} dropdown-header ${isOpen() ? "open" : ""}`}
				onclick={() => {
					if (props.onHeaderClick != undefined && !isOpen()) {
						props.onHeaderClick();
					} else {
						if (isOpen()) {
							close();
						} else {
							setIsOpen(true);
						}
					}
				}}
				style={`${isOpen() && isSearchable ? "justify-content:flex-start" : ""
					}`}
				onmouseenter={() => setIsHeaderHovered(true)}
				onmouseleave={() => setIsHeaderHovered(false)}
			>
				<Switch>
					<Match when={!isOpen() || !isSearchable}>{headerContents()}</Match>
					<Match when={isOpen() && isSearchable}>
						<input
							type="text"
							class="dropdown-search"
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
							onfocusout={() => {
								if (props.startOpen == true) {
									close();
								}
							}}
							ref={searchElement}
						/>
						<div class="cont dropdown-search-icon">
							<Icon icon={Search} size="1rem" />
						</div>
					</Match>
				</Switch>
				<Show when={showArrow}>
					<div
						class={`cont dropdown-arrow ${isOpen() ? "open" : ""}`}
						onclick={(e) => {
							if (props.onHeaderClick != undefined) {
								if (isOpen()) {
									close();
								} else {
									setIsOpen(true);
								}
								e.preventDefault();
								e.stopPropagation();
							}
						}}
					>
						<Show
							when={isOpen()}
							fallback={<Icon icon={AngleRight} size="1rem" />}
						>
							<Icon icon={AngleRight} size="1rem" />
						</Show>
					</div>
				</Show>
			</div>
			<div
				class="dropdown-options"
				style={`${!isOpen()
					? "max-height:0px;border-top-width:0px;border-bottom-width:0px;outline-width:0;transition: max-height 0.25s, border-width 0.25s step-end"
					: `max-height:${openedHeight()}`
					};${zIndex};${props.optionsWidth != undefined ? `width:${props.optionsWidth}` : ""
					}`}
				onmouseenter={() => setAreOptionsHovered(true)}
				onmouseleave={() => setAreOptionsHovered(false)}
			>
				<Show when={allowEmpty}>
					<DropdownOption
						option={{
							value: undefined,
							contents: "None",
						}}
						onSelect={selectFunction}
						isSelected={props.selected == undefined}
						isFirst={true}
						isLast={props.options.length == 0}
						class={props.optionClass}
					/>
				</Show>
				<For each={props.options}>
					{(option, index) => {
						let isVisible = createMemo(() => {
							return (
								props.customSearchFunction != undefined ||
								search() == undefined ||
								search()!.length == 0 ||
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
									isFirst={!allowEmpty && index() == 0}
									isLast={index() == lastIndex()}
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
	isSearchable?: boolean;
	customSearchFunction?: (search: string | undefined) => void;
	startOpen?: boolean;
	previewText?: JSX.Element;
	optionsWidth?: string;
	showArrow?: boolean;
	onHeaderClick?: () => void;
	onClose?: () => void;
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
		if (props.option.backgroundColor != undefined) {
			return props.option.backgroundColor;
		} else if (props.isSelected) {
			return "var(--bg)";
		} else if (isHovered()) {
			return "var(--bg3)";
		} else {
			return "var(--bg2)";
		}
	};

	let contents = (
		<div
			class={`cont bubble-hover dropdown-option ${props.class == undefined ? "" : props.class
				} ${props.isSelected ? "selected" : ""} ${props.isLast ? "last" : "not-last"
				}`}
			style={`color:${textColor()};background-color:${backgroundColor()};${props.isFirst
				? "border-top-left-radius:var(--round2);border-top-right-radius:var(--round2)"
				: ""
				}${props.isLast
					? "border-bottom-left-radius:var(--round2);border-bottom-right-radius:var(--round2)"
					: ""
				}`}
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
	isSelectable?: boolean;
	backgroundColor?: string;
}
