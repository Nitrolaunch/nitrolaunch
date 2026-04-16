import { createReaction, createSignal, For, Match, Switch } from "solid-js";
import "./Control.css";
import Tip from "../dialog/Tip";
import SlideSwitch from "./SlideSwitch";
import Dropdown, { Option } from "./select/Dropdown";
import InlineSelect from "./select/InlineSelect";
import PathSelect from "./text/PathSelect";
import EditableList from "./text/EditableList";
import {
	ControlledConfig,
	InstanceConfig,
} from "../../pages/instance/read_write";
import DeriveIndicator from "../../pages/instance/DeriveIndicator";
import { canonicalizeListOrSingle } from "../../utils/values";

export default function Control(props: ControlProps) {
	let [value, setValue] = createSignal<any>(props.initialValue);

	// We don't want to set the value and dirty the config on the initial value set
	let track = createReaction(() => {
		props.setValue(props.control.id, value());

		track(() => value());
	});

	track(() => value());

	let deriveIndicator = (
		<DeriveIndicator
			parentConfigs={props.parentConfigs}
			currentValue={value()}
			property={(template) =>
				new ControlledConfig(template).getControl(props.control.id)
			}
		/>
	);

	let elem = () => {
		switch (props.control.schema.type) {
			case "boolean":
				return (
					<SlideSwitch
						enabled={value()}
						onToggle={() => setValue(!value())}
						enabledColor={
							props.control.color == undefined
								? "var(--instance)"
								: props.control.color
						}
						disabledColor="var(--fg3)"
					/>
				);
			case "choice":
				if (props.control.schema.dropdown) {
					return (
						<Dropdown
							options={props.control.schema.variants.map((x) => {
								return { value: x.id, contents: x.name, color: x.color };
							})}
							selected={value()}
							onChange={props.control.schema.multiple ? undefined : setValue}
							onChangeMulti={
								props.control.schema.multiple ? setValue : undefined
							}
						/>
					);
				} else {
					return (
						<InlineSelect
							options={props.control.schema.variants.map((x) => {
								return { value: x.id, contents: x.name, color: x.color };
							})}
							selected={value()}
							onChange={props.control.schema.multiple ? undefined : setValue}
							onChangeMulti={
								props.control.schema.multiple ? setValue : undefined
							}
							connected={false}
						/>
					);
				}
			case "number":
				return (
					<input
						type="number"
						value={value()}
						onchange={(e) => {
							e.preventDefault();
							setValue((e.target.value as any) * 1);
						}}
						step={props.control.schema.step}
						min={props.control.schema.min}
						max={props.control.schema.max}
					/>
				);
			case "string_list":
				return (
					<EditableList
						items={value() == undefined ? [] : value()}
						setItems={setValue}
						reorderable
					/>
				);
			case "string":
				return (
					<input
						type="text"
						value={value()}
						onchange={(e) => {
							e.preventDefault();
							setValue(e.target.value);
						}}
					/>
				);
			case "path":
				return <PathSelect path={value()} setPath={setValue} />;
			case "keybind":
				return (
					<Dropdown
						options={keybindOptions}
						selected={value()}
						onChange={setValue}
					/>
				);
			case "list":
				let isMap = props.control.schema.is_map;
				let fields = props.control.schema.fields;
				let listValue = () =>
					value() == undefined
						? []
						: isMap
							? Object.keys(value())
							: canonicalizeListOrSingle(value());

				return (
					<For each={listValue()}>
						{(item, i) => {
							let itemValue = isMap ? value()[item] : item;

							return (
								<div class="cont col list-control-item">
									<For each={fields}>
										{(control) => {
											return (
												<Control
													control={control}
													initialValue={itemValue}
													setValue={(id, newValue) => {
														if (isMap) {
															setValue((current) => {
																let map = new ControlledConfig(current[item]);
																map.setControl(id, newValue);
																return { ...current };
															});
														} else {
															setValue((current) => {
																let map = new ControlledConfig(current[i()]);
																map.setControl(id, newValue);
																return [...current];
															});
														}
													}}
													side={props.side}
													parentConfigs={props.parentConfigs}
												/>
											);
										}}
									</For>
								</div>
							);
						}}
					</For>
				);
			default:
				return <div>Unimplemented</div>;
		}
	};

	return (
		<div class="cont col fullwidth" style="align-items:flex-start">
			<label class="cont start label">
				{props.control.name.toLocaleUpperCase()}
				{deriveIndicator}
			</label>
			<Switch>
				<Match when={props.control.description == undefined}>{elem()}</Match>
				<Match when={props.control.description != undefined}>
					<Tip tip={props.control.description} side="top" fullwidth>
						{elem()}
					</Tip>
				</Match>
			</Switch>
		</div>
	);
}

export interface ControlProps {
	control: ControlData;
	initialValue: any;
	setValue: (id: string, value: any) => void;
	side?: "client" | "server";
	parentConfigs: InstanceConfig[];
}

// A serializable value with a schema
export interface ControlData {
	id: string;
	name: string;
	schema: ControlSchema;
	default?: any;
	description?: string;
	color?: string;
	section?: string;
	always_serialize: boolean;
	side?: "client" | "server";
}

export type ControlSchema =
	| { type: "boolean" }
	| {
			type: "choice";
			variants: Variant[];
			dropdown: boolean;
			multiple: boolean;
	  }
	| {
			type: "string";
			lowercase: boolean;
	  }
	| { type: "path" }
	| {
			type: "number";
			min?: number;
			max?: number;
			step: number;
			slider: boolean;
	  }
	| {
			type: "string_list";
	  }
	| {
			type: "optional";
			value: ControlSchema;
	  }
	| {
			type: "list";
			fields: ControlData[];
			is_map: boolean;
	  }
	| {
			type: "keybind";
	  };

// Variant of a choice control
export interface Variant {
	id: string;
	name: string;
	color?: string;
}

const keybindOptions: Option[] = [
	{ value: "unbound", contents: "Unbound" },

	{ value: "mouse_left", contents: "Mouse Left" },
	{ value: "mouse_right", contents: "Mouse Right" },
	{ value: "mouse_middle", contents: "Mouse Middle" },
	{ value: "mouse4", contents: "Mouse 4" },
	{ value: "mouse5", contents: "Mouse 5" },
	{ value: "mouse6", contents: "Mouse 6" },
	{ value: "mouse7", contents: "Mouse 7" },
	{ value: "mouse8", contents: "Mouse 8" },

	...Array.from({ length: 10 }, (_, i) => ({
		value: `num${i}`,
		contents: `Num ${i}`,
	})),

	...Array.from("ABCDEFGHIJKLMNOPQRSTUVWXYZ").map((c) => ({
		value: c.toLowerCase(),
		contents: c,
	})),

	...Array.from({ length: 25 }, (_, i) => ({
		value: `f${i + 1}`,
		contents: `F${i + 1}`,
	})),

	{ value: "num_lock", contents: "Num Lock" },

	...Array.from({ length: 10 }, (_, i) => ({
		value: `numpad${i}`,
		contents: `Numpad ${i}`,
	})),

	{ value: "numpad_add", contents: "Numpad Add" },
	{ value: "numpad_decimal", contents: "Numpad Decimal" },
	{ value: "numpad_enter", contents: "Numpad Enter" },
	{ value: "numpad_equal", contents: "Numpad Equal" },
	{ value: "numpad_multiply", contents: "Numpad Multiply" },
	{ value: "numpad_divide", contents: "Numpad Divide" },
	{ value: "numpad_subtract", contents: "Numpad Subtract" },

	{ value: "down", contents: "Down Arrow" },
	{ value: "left", contents: "Left Arrow" },
	{ value: "right", contents: "Right Arrow" },
	{ value: "up", contents: "Up Arrow" },

	{ value: "apostrophe", contents: "Apostrophe (')" },
	{ value: "backslash", contents: "Backslash (\\" },
	{ value: "comma", contents: "Comma (,)" },
	{ value: "equal", contents: "Equal (=)" },
	{ value: "grave_accent", contents: "Grave Accent (`)" },
	{ value: "left_bracket", contents: "Left Bracket ([)" },
	{ value: "right_bracket", contents: "Right Bracket (])" },
	{ value: "minus", contents: "Minus (-)" },
	{ value: "period", contents: "Period (.)" },
	{ value: "semicolon", contents: "Semicolon (;)" },
	{ value: "slash", contents: "Slash (/)" },

	{ value: "space", contents: "Space" },
	{ value: "tab", contents: "Tab" },

	{ value: "left_alt", contents: "Left Alt" },
	{ value: "right_alt", contents: "Right Alt" },
	{ value: "left_shift", contents: "Left Shift" },
	{ value: "right_shift", contents: "Right Shift" },
	{ value: "left_control", contents: "Left Ctrl" },
	{ value: "right_control", contents: "Right Ctrl" },
	{ value: "left_system", contents: "Left Meta / Win" },
	{ value: "right_system", contents: "Right Meta / Win" },

	{ value: "enter", contents: "Enter" },
	{ value: "escape", contents: "Escape" },
	{ value: "backspace", contents: "Backspace" },
	{ value: "delete", contents: "Delete" },

	{ value: "home", contents: "Home" },
	{ value: "end", contents: "End" },
	{ value: "insert", contents: "Insert" },
	{ value: "page_down", contents: "Page Down" },
	{ value: "page_up", contents: "Page Up" },

	{ value: "caps_lock", contents: "Caps Lock" },
	{ value: "pause", contents: "Pause" },
	{ value: "scroll_lock", contents: "Scroll Lock" },
	{ value: "menu", contents: "Menu" },
	{ value: "print_screen", contents: "Print Screen" },

	{ value: "world1", contents: "World 1" },
	{ value: "world2", contents: "World 2" },
];
