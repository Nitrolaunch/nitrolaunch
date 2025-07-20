import { createEffect, createSignal, Match, Switch } from "solid-js";
import Icon from "../Icon";
import { Link, LinkBroken } from "../../icons";
import "./LinkedInputs.css";

export default function LinkedInputs(props: LinkedInputsProps) {
	let suffix = props.suffix == undefined ? "" : props.suffix;

	let [isLinked, setIsLinked] = createSignal(
		props.value1 == undefined || props.value2 == undefined
	);

	createEffect(() => {
		if (props.value1 != undefined && isLinked()) {
			props.setValue2(props.value1 * props.ratio);
		}
	});

	createEffect(() => {
		if (props.value2 != undefined && isLinked()) {
			props.setValue1(props.value2 / props.ratio);
		}
	});

	return (
		<div class="cont col fullwidth linked-inputs">
			<div class="fullwidth linked-inputs-labels">
				<div class="cont start label linked-input-label">{props.label1}</div>
				<div
					class="cont end label linked-input-label"
					style="margin-right: 0.2rem"
				>
					{props.label2}
				</div>
			</div>
			<div class="fullwidth linked-inputs-inputs">
				<div class="cont col linked-input-container">
					<div class={`cont linked-input ${suffix}`}>
						<input
							type="number"
							value={props.value1}
							onkeyup={(e: any) => {
								props.setValue1(e.target.value * 1);
								if (props.onChange != undefined) {
									props.onChange();
								}
							}}
						/>
					</div>
				</div>
				<div
					class="cont linked-inputs-link"
					onclick={() => setIsLinked(!isLinked())}
				>
					<Switch>
						<Match when={isLinked()}>
							<Icon icon={Link} size="1rem" />
						</Match>
						<Match when={!isLinked()}>
							<Icon icon={LinkBroken} size="1rem" />
						</Match>
					</Switch>
				</div>
				<div class="cont col linked-input-container">
					<div class={`cont linked-input ${suffix}`}>
						<input
							type="number"
							value={props.value2}
							onkeyup={(e: any) => {
								props.setValue2(e.target.value * 1);
								if (props.onChange != undefined) {
									props.onChange();
								}
							}}
						/>
					</div>
				</div>
			</div>
		</div>
	);
}

export interface LinkedInputsProps {
	value1: number | undefined;
	value2: number | undefined;
	setValue1: (value: number) => void;
	setValue2: (value: number) => void;
	onChange?: () => void;
	ratio: number;
	label1: string;
	label2: string;
	suffix?: "px" | "mb";
}
