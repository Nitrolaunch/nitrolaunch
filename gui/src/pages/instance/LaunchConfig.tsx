import Tip from "../../components/dialog/Tip";
import LinkedInputs from "../../components/input/LinkedInputs";
import DeriveIndicator from "./DeriveIndicator";
import {
	getDerivedValue,
	getJavaDisplayName,
	InstanceConfig,
	JavaType,
} from "./read_write";
import EditableList from "../../components/input/EditableList";
import InlineSelect from "../../components/input/InlineSelect";

export default function LaunchConfig(props: LaunchConfigProps) {
	return (
		<div class="fields">
			<div class="cont start label">
				<label for="java">JAVA</label>
				<DeriveIndicator
					parentConfigs={props.parentConfigs}
					currentValue={props.java}
					property={(x) => {
						x.launch == undefined ? undefined : x.launch.java;
					}}
				/>
			</div>
			<Tip tip="The Java installation to use" fullwidth>
				<InlineSelect
					onChange={(x) => {
						props.setJava(x);
						props.onChange();
					}}
					selected={props.java}
					options={[
						undefined,
						"auto",
						"system",
						"adoptium",
						"zulu",
						"graalvm",
					].map((x) => {
						return {
							value: x,
							contents: (
								<div
									class={`cont ${
										props.java == undefined &&
										getDerivedValue(props.parentConfigs, (x) =>
											x.launch == undefined ? undefined : x.launch.java
										) == x
											? "derived-option"
											: ""
									}`}
								>
									{x == undefined ? "Unset" : getJavaDisplayName(x)}
								</div>
							),
							tip: x == undefined ? "Inherit from the profile" : getJavaTip(x),
						};
					})}
					columns={4}
					allowEmpty={false}
					connected={false}
				/>
			</Tip>
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
	java?: JavaType;
	setJava: (value: JavaType | undefined) => void;
	initMemory?: number;
	maxMemory?: number;
	setInitMemory: (value: number | undefined) => void;
	setMaxMemory: (value: number | undefined) => void;
	envVars: string[];
	setEnvVars: (value: string[]) => void;
	onChange: () => void;
	parentConfigs: InstanceConfig[];
}

function getJavaTip(x: JavaType) {
	if (x == "auto") {
		return "Finds Java on your system if available, and downloads it otherwise";
	} else if (x == "system") {
		return "Finds Java on your system";
	} else if (x == "adoptium") {
		return "Downloads Adoptium Temurin JDK";
	} else if (x == "zulu") {
		return "Downloads Azul Zulu JDK";
	} else if (x == "graalvm") {
		return "Downloads GraalVM JDK";
	} else {
		return undefined;
	}
}
