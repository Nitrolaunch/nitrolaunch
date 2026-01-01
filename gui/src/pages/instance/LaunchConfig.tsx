import Tip from "../../components/dialog/Tip";
import LinkedInputs from "../../components/input/text/LinkedInputs";
import DeriveIndicator from "./DeriveIndicator";
import { getDerivedValue, InstanceConfig, JavaType } from "./read_write";
import EditableList from "../../components/input/text/EditableList";
import InlineSelect from "../../components/input/select/InlineSelect";
import PathSelect from "../../components/input/text/PathSelect";
import { createResource, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { errorToast } from "../../components/dialog/Toasts";

export default function LaunchConfig(props: LaunchConfigProps) {
	let [javaTypes, _] = createResource(
		async () => {
			let baseTypes: JavaTypeInfo[] = [
				{ id: undefined, name: "Unset", color: "var(--fg)" },
				{ id: "auto", name: "Auto", color: "var(--fg)" },
				{ id: "system", name: "System", color: "var(--fg)" },
				{ id: "adoptium", name: "Adoptium", color: "var(--fg)" },
			];
			try {
				let pluginTypes: JavaTypeInfo[] = await invoke(
					"get_supported_java_types"
				);
				return baseTypes.concat(pluginTypes);
			} catch (e) {
				errorToast("Failed to get available Java types: " + e);
				return baseTypes;
			}
		},
		{ initialValue: [] }
	);

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
			<InlineSelect
				onChange={(x) => {
					props.setJava(x);
					props.onChange();
				}}
				selected={props.java}
				options={javaTypes()
					.concat({ id: "custom", name: "Custom", color: "var(--fg)" })
					.map((x) => {
						return {
							value: x.id,
							contents: (
								<div
									class={`cont ${
										props.java == undefined &&
										getDerivedValue(props.parentConfigs, (x) =>
											x.launch == undefined ? undefined : x.launch.java
										) == x.id
											? "derived-option"
											: ""
									}`}
								>
									{x.name}
								</div>
							),
							color: x.color,
							tip:
								x.id == undefined
									? "Inherit from the template"
									: getJavaTip(x.id),
						};
					})}
				columns={4}
				allowEmpty={false}
				connected={false}
			/>
			<Show
				when={
					!javaTypes().some((x) => x.id == props.java) &&
					props.java != undefined
				}
			>
				<Tip tip="Path to custom Java installation" fullwidth side="top">
					<div class="cont fullwidth" id="launch-custom-java">
						<PathSelect
							path={props.java == "custom" ? "" : props.java}
							setPath={(path) => {
								props.setJava(path);
								props.onChange();
							}}
						/>
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
				side="top"
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
				side="top"
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
			<Tip tip="Arguments for the JVM" side="top" fullwidth>
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
				side="top"
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
	} else if (x == "custom") {
		return "Select a custom path on your system";
	} else {
		return undefined;
	}
}

interface JavaTypeInfo {
	id?: string;
	name: string;
	color: string;
}
