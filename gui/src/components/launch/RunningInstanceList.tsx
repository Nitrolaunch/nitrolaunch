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
import { errorToast } from "../dialog/Toasts";

// Displays a list of instance icons that can be interacted with
export default function RunningInstanceList(props: RunningInstanceListProps) {
	let navigate = useNavigate();

	let [instances, setInstances] = createSignal<RunningInstanceEntry[]>([]);

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

	let [hover, setHover] = createSignal<HoverData | undefined>();

	return (
		<div class="cont running-instance-list">
			<Show when={hover() != undefined}>
				<div class="cont col pop-in-fast" id="running-instance-list-tip">
					{hover()!.name}
					<Show when={hover()!.user != undefined}>
						<span style="color:var(--fg2);font-size:0.9rem">
							User: {hover()!.user}
						</span>
					</Show>
					<span style="color:var(--error);font-size:0.9rem">
						Right click to kill
					</span>
				</div>
			</Show>
			<For each={instances()}>
				{(instance) => {
					let info = () =>
						instanceInfo().find((x) => x.id == instance.instance_id);

					let name =
						info() != undefined && info()!.name != undefined
							? info()!.name!
							: instance.instance_id;

					return (
						<img
							src={getInstanceIconSrc(
								info() == undefined ? undefined : info()!.icon
							)}
							class="running-instance-list-icon"
							onclick={() => {
								navigate(`/instance/${instance.instance_id}`);
							}}
							onmouseenter={() => setHover({ name: name, user: instance.user })}
							onmouseleave={() => setHover(undefined)}
							onerror={(e: any) =>
								(e.target.src = "/icons/default_instance.png")
							}
							oncontextmenu={(e) => {
								e.preventDefault();

								try {
									invoke("kill_instance", {
										instance: instance.instance_id,
										user: instance.user,
									});
								} catch (e) {
									errorToast("Failed to kill instance: " + e);
								}
							}}
						/>
					);
				}}
			</For>
		</div>
	);
}

export interface RunningInstanceListProps {}

interface HoverData {
	name: string;
	user?: string;
}

export interface RunningInstancesEvent {
	running_instances: RunningInstanceEntry[];
}

export interface RunningInstanceEntry {
	instance_id: string;
	user?: string;
}
