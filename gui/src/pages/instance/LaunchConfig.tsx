import Tip from "../../components/dialog/Tip";
import LinkedInputs from "../../components/input/LinkedInputs";
import DeriveIndicator from "./DeriveIndicator";
import { InstanceConfig } from "./read_write";
import EditableList from "../../components/input/EditableList";

export default function LaunchConfig(props: LaunchConfigProps) {
	return (
		<div class="fields">
			<div class="cont start">
				<label for="launch-memory" class="label">
					JVM MEMORY
				</label>
				<DeriveIndicator
					parentConfigs={props.parentConfigs}
					currentValue={props.initMemory}
					property={(x) =>
						x.launch == undefined ? undefined : x.launch.memory
					}
				/>
			</div>
			<Tip
				tip="The amount of memory to allocate to the JVM, in megabytes"
				fullwidth
			>
				<LinkedInputs
					value1={props.initMemory}
					value2={props.maxMemory}
					setValue1={props.setInitMemory}
					setValue2={props.setMaxMemory}
					label1="INITIAL"
					label2="MAX"
					ratio={1}
					suffix="mb"
					onChange={props.onChange}
				/>
			</Tip>
			<div class="cont start">
				<label for="launch-env" class="label">
					ENVIRONMENT VARIABLES
				</label>
				<DeriveIndicator
					parentConfigs={props.parentConfigs}
					currentValue={props.envVars.join(", ")}
					property={(x) => (x.launch == undefined ? undefined : x.launch.env)}
				/>
			</div>
			<Tip
				tip="Environment variables for the game. Each entry should look like KEY=value"
				fullwidth
			>
				<EditableList
					items={props.envVars}
					setItems={(x) => {
						props.setEnvVars(x);
						props.onChange();
					}}
				/>
			</Tip>
		</div>
	);
}

export interface LaunchConfigProps {
	initMemory?: number;
	maxMemory?: number;
	setInitMemory: (value: number | undefined) => void;
	setMaxMemory: (value: number | undefined) => void;
	envVars: string[];
	setEnvVars: (value: string[]) => void;
	onChange: () => void;
	parentConfigs: InstanceConfig[];
}
