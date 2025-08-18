import { Match, Show, Switch } from "solid-js";
import "./DeriveIndicator.css";
import { getDerivedValue, InstanceConfig } from "./read_write";
import { undefinedEmpty } from "../../utils/values";

export default function DeriveIndicator(props: DeriveIndicatorProps) {
	let emptyUndefinedTrue =
		props.emptyUndefined == undefined ? false : props.emptyUndefined;

	let displayValue =
		props.displayValue == undefined ? false : props.displayValue;

	let value = () => getDerivedValue(props.parentConfigs, props.property);

	let visible = () =>
		(emptyUndefinedTrue
			? undefinedEmpty(props.currentValue) == undefined
			: props.currentValue == undefined) && value() != undefined;

	return (
		<Show when={visible()}>
			<div class="cont derive-indicator">
				<Switch>
					<Match when={displayValue}>DERIVED VALUE: {value()}</Match>
					<Match when={!displayValue}>
						<div class="derive-indicator-asterisk">*</div>DERIVED
					</Match>
				</Switch>
			</div>
		</Show>
	);
}

export interface DeriveIndicatorProps {
	parentConfigs: InstanceConfig[];
	currentValue: any | undefined;
	property: (profile: InstanceConfig) => any | undefined;
	emptyUndefined?: boolean;
	displayValue?: boolean;
}
