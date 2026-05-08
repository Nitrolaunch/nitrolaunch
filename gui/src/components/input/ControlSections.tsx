import Control, { ControlData } from "./Control";
import { createMemo, For, Show } from "solid-js";
import CollapsableSection from "../utility/CollapsableSection";
import { InstanceConfig } from "../../pages/instance/read_write";

export default function ControlSections(props: ControlSectionsProps) {
	let sections = createMemo(() => getControlSections(props.controls));

	return (
		<div class="cont col fullwidth control-sections">
			<For each={Object.keys(sections()).filter((x) => x != "_default")}>
				{(section) => {
					let controls = () => sections()[section];
					return (
						<CollapsableSection title={section}>
							<div class="cont col fields">
								<For each={controls()}>
									{(control) => {
										let initialValue = props.getInitialValue(control.id);
										let isShown = () => {
											if (
												props.side == undefined ||
												control.side == undefined
											) {
												return true;
											} else {
												return props.side == control.side;
											}
										};

										return (
											<Show when={isShown()}>
												<Control
													control={control}
													initialValue={
														initialValue == undefined
															? control.default
															: initialValue
													}
													setValue={props.setValue}
													side={props.side}
													parentConfigs={props.parentConfigs}
												/>
											</Show>
										);
									}}
								</For>
							</div>
						</CollapsableSection>
					);
				}}
			</For>
			<div class="cont col fields">
				<For each={sections()["_default"]}>
					{(control) => {
						let initialValue = props.getInitialValue(control.id);
						let isShown = () => {
							if (props.side == undefined || control.side == undefined) {
								return true;
							} else {
								return props.side == control.side;
							}
						};

						return (
							<Show when={isShown()}>
								<Control
									control={control}
									initialValue={
										initialValue == undefined ? control.default : initialValue
									}
									setValue={props.setValue}
									side={props.side}
									parentConfigs={props.parentConfigs}
								/>
							</Show>
						);
					}}
				</For>
			</div>
		</div>
	);
}

export interface ControlSectionsProps {
	controls: ControlData[];
	getInitialValue: (id: string) => any;
	setValue: (id: string, value: any) => void;
	side?: "client" | "server";
	parentConfigs: InstanceConfig[];
}

export function getControlSections(controls: ControlData[]): {
	[key: string]: ControlData[];
} {
	let sections: { [key: string]: ControlData[] } = {};
	sections["_default"] = [];

	for (let control of controls) {
		if (control.section == undefined) {
			sections["_default"].push(control);
			continue;
		}

		if (sections[control.section] == undefined) {
			sections[control.section] = [];
		}
		sections[control.section].push(control);
	}

	return sections;
}
