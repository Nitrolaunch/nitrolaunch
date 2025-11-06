import {
	createResource,
	createSignal,
	For,
	onCleanup,
	onMount,
	Show,
} from "solid-js";
import "./RunningInstanceList.css";
import { Event, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { InstanceInfo } from "../../types";
import { getInstanceIconSrc } from "../../utils";
import { useNavigate } from "@solidjs/router";

// Displays a list of instance icons that can be interacted with
export default function RunningInstanceList(props: RunningInstanceListProps) {
	let navigate = useNavigate();

	let [instances, setInstances] = createSignal<string[]>([]);

	onMount(() => {
		invoke("update_running_instances");
	});

	let [eventUnlisten, _] = createResource(async () => {
		return await listen(
			"nitro_update_running_instances",
			(e: Event<RunningInstancesEvent>) => {
				setInstances(e.payload.running_instances);
			}
		);
	});

	onCleanup(() => {
		if (eventUnlisten() != undefined) {
			eventUnlisten()!();
		}
	});

	let [instanceInfo, __] = createResource(
		() => instances(),
		async () => {
			return (await invoke("get_instances")) as InstanceInfo[];
		},
		{ initialValue: [] }
	);

	let [hoveredName, setHoveredName] = createSignal<string | undefined>(
		undefined
	);

	return (
		<div class="cont running-instance-list">
			<Show when={hoveredName() != undefined}>
				<div class="cont pop-in-fast" id="running-instance-list-tip">
					{hoveredName()}
				</div>
			</Show>
			<For each={instances()}>
				{(instance) => {
					let info = () => instanceInfo().find((x) => x.id == instance);

					let name =
						info() != undefined && info()!.name != undefined
							? info()!.name!
							: instance;

					return (
						<img
							src={getInstanceIconSrc(
								info() == undefined ? undefined : info()!.icon
							)}
							class="running-instance-list-icon"
							onclick={() => {
								navigate(`/instance/${instance}`);
							}}
							onmouseenter={() => setHoveredName(name)}
							onmouseleave={() => setHoveredName(undefined)}
						/>
					);
				}}
			</For>
		</div>
	);
}

export interface RunningInstanceListProps {}

export interface RunningInstancesEvent {
	running_instances: string[];
}
