import Tip from "../../components/dialog/Tip";
import LinkedInputs from "../../components/input/text/LinkedInputs";
import DeriveIndicator from "./DeriveIndicator";
import {
	getDerivedValue,
	getJavaDisplayName,
	InstanceConfig,
	JavaType,
} from "./read_write";
import EditableList from "../../components/input/text/EditableList";
import InlineSelect from "../../components/input/select/InlineSelect";
import PathSelect from "../../components/input/text/PathSelect";
import { Show } from "solid-js";

const JAVA_OPTIONS = [
	undefined,
	"auto",
	"system",
	"adoptium",
	"zulu",
	"graalvm",
];

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
			<Tip tip="The Java installation to use. Defaults to 'Auto'" fullwidth>
				<InlineSelect
					onChange={(x) => {
						props.setJava(x);
						props.onChange();
					}}
					selected={props.java}
					options={JAVA_OPTIONS.concat("custom").map((x) => {
						return {
							value: x,
							contents: (
								<div
									class={`cont ${props.java == undefined &&
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
			<Show when={!JAVA_OPTIONS.includes(props.java) && props.java != undefined}>
				<Tip tip="Path to custom Java installation" fullwidth side="right">
					<div class="cont fullwidth" id="launch-custom-java">
						<PathSelect path={props.java == "custom" ? "" : props.java} setPath={(path) => { props.setJava(path); props.onChange() }} />
					</div>
				</Tip>
			</Show>
			<div class="cont start label">
				<label for="launch-memory">JVM MEMORY</label>
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
			<div class="cont start label">
				<label for="launch-env">ENVIRONMENT VARIABLES</label>
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
			<div class="cont start label">
				<label for="launch-jvm-args">JVM ARGUMENTS</label>
				<DeriveIndicator
					parentConfigs={props.parentConfigs}
					currentValue={props.jvmArgs.join(", ")}
					property={(x) =>
						x.launch == undefined || x.launch.args == undefined
							? undefined
							: x.launch.args.jvm
					}
				/>
			</div>
			<Tip tip="Arguments for the JVM" fullwidth>
				<EditableList
					items={props.jvmArgs}
					setItems={(x) => {
						props.setJvmArgs(x);
						props.onChange();
					}}
				/>
			</Tip>
			<div class="cont start label">
				<label for="launch-game-args">GAME ARGUMENTS</label>
				<DeriveIndicator
					parentConfigs={props.parentConfigs}
					currentValue={props.gameArgs.join(", ")}
					property={(x) =>
						x.launch == undefined || x.launch.args == undefined
							? undefined
							: x.launch.args.game
					}
				/>
			</div>
			<Tip
				tip="Arguments for Minecraft. Most arguments are for the JVM instead."
				fullwidth
			>
				<EditableList
					items={props.gameArgs}
					setItems={(x) => {
						props.setGameArgs(x);
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
	jvmArgs: string[];
	gameArgs: string[];
	setJvmArgs: (value: string[]) => void;
	setGameArgs: (value: string[]) => void;
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
