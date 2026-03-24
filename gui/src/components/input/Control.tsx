import { createReaction, createSignal, Match, Switch } from "solid-js";
import "./Control.css";
import Tip from "../dialog/Tip";
import SlideSwitch from "./SlideSwitch";
import Dropdown from "./select/Dropdown";
import InlineSelect from "./select/InlineSelect";
import PathSelect from "./text/PathSelect";
import EditableList from "./text/EditableList";

export default function Control(props: ControlProps) {
	let [value, setValue] = createSignal<any>(props.initialValue);

	// We don't want to set the value and dirty the config on the initial value set
	let track = createReaction(() => {
		props.setValue(props.control.id, value());

		track(() => value());
	});

	track(() => value());

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
			default:
				return <div>Unimplemented</div>;
		}
	};

	return (
		<div class="cont col fullwidth" style="align-items:flex-start">
			<label class="label">{props.control.name.toLocaleUpperCase()}</label>
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
	  };

// Variant of a choice control
export interface Variant {
	id: string;
	name: string;
	color?: string;
}
