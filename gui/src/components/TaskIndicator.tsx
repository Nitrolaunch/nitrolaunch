import { Event, listen } from "@tauri-apps/api/event";
import {
	createMemo,
	createResource,
	createSignal,
	For,
	Match,
	onCleanup,
	onMount,
	Show,
	Switch,
} from "solid-js";
import "./TaskIndicator.css";
import { Delete, Spinner } from "../icons";
import {
	errorToast,
	messageToast,
	removeThisToast,
	successToast,
	warningToast,
} from "./dialog/Toasts";
import { beautifyString } from "../utils";
import { invoke } from "@tauri-apps/api";
import IconButton from "./input/IconButton";

export default function TaskIndicator(props: TaskIndicatorProps) {
	// Map of tasks to messages
	let [messages, setMessages] = createSignal<TaskMap>({});
	let [taskCount, setTaskCount] = createSignal(0);
	let [taskName, setTaskName] = createSignal<string | undefined>(undefined);
	let [color, setColor] = createSignal<Color>("disabled");

	// The task visible in the popup
	let [selectedTask, setSelectedTask] = createSignal<number | undefined>(
		undefined
	);

	function createTask(task: string) {
		if (messages()[task] == undefined) {
			setTaskCount((taskCount) => taskCount + 1);
		}
		if (taskCount() == 1) {
			setTaskName(getTaskDisplayName(task));
			setColor(getTaskColor(task));
		} else {
			setColor("running");
		}
		setMessages((messages) => {
			messages[task] = {
				id: task,
				messages: [],
				sectionStack: [],
				processName: undefined,
				nextMessageIsProcess: false,
				nextMessageIsSection: true,
			};
			return messages;
		});
	}

	let [eventUnlistens, _] = createResource(async () => {
		let unlisten1 = listen(
			"nitro_output_create_task",
			(event: Event<string>) => {
				createTask(event.payload);
			}
		);

		(window as any).foo = messages;
		(window as any).bar = taskCount;
		(window as any).baz = selectedTask;

		let unlisten2 = listen(
			"nitro_output_message",
			(event: Event<MessageEvent>) => {
				if (event.payload.type == MessageType.Warning) {
					warningToast(event.payload.message);
					return;
				} else if (event.payload.type == MessageType.Error) {
					errorToast(event.payload.message);
					return;
				}

				if (event.payload.task != undefined) {
					setMessages((messages) => {
						if (messages[event.payload.task!] != undefined) {
							let task = messages[event.payload.task!]!;

							// Handle starts of processes and sections
							if (event.payload.type == MessageType.StartProcess) {
								if (task.nextMessageIsProcess) {
									task.processName = event.payload.message;
								}
							} else if (event.payload.type == MessageType.Header) {
								if (task.nextMessageIsSection) {
									task.sectionStack.push(event.payload.message);
								}
							} else {
								// Add the message
								task.messages.push({
									message: event.payload.message,
									messageType: event.payload.type,
								});
							}
						}
						return messages;
					});
				}
			}
		);

		let unlisten3 = listen(
			"nitro_output_finish_task",
			(event: Event<string>) => {
				if (messages()[event.payload] != undefined) {
					setTaskCount((taskCount) => taskCount - 1);
				}

				// TODO: Keep the same task focused
				if (selectedTask() != undefined && selectedTask()! >= taskCount() - 1) {
					if (taskCount() == 0) {
						setSelectedTask(undefined);
					} else {
						setSelectedTask(0);
					}
				}

				setMessages((messages) => {
					delete messages[event.payload];
					return messages;
				});
				if (taskCount() == 0) {
					setColor("disabled");
				} else if (taskCount() == 1) {
					setTaskName(getTaskDisplayName(Object.keys(messages())[0]!));
				}
			}
		);

		let unlisten4 = listen(
			"nitro_output_start_process",
			(event: Event<string | undefined>) => {
				if (event.payload != undefined) {
					setMessages((messages) => {
						if (messages[event.payload!] != undefined) {
							messages[event.payload!]!.nextMessageIsProcess = true;
						}

						return messages;
					});
				}
			}
		);

		let unlisten5 = listen(
			"nitro_output_end_process",
			(event: Event<string | undefined>) => {
				if (event.payload != undefined) {
					setMessages((messages) => {
						if (messages[event.payload!] != undefined) {
							messages[event.payload!]!.processName = undefined;
						}

						return messages;
					});
				}
			}
		);

		let unlisten6 = listen(
			"nitro_output_start_section",
			(event: Event<string | undefined>) => {
				if (event.payload != undefined) {
					setMessages((messages) => {
						if (messages[event.payload!] != undefined) {
							messages[event.payload!]!.nextMessageIsSection = true;
						}

						return messages;
					});
				}
			}
		);

		let unlisten7 = listen(
			"nitro_output_end_section",
			(event: Event<string | undefined>) => {
				if (event.payload != undefined) {
					console.log("Pop");
					setMessages((messages) => {
						if (messages[event.payload!] != undefined) {
							messages[event.payload!]!.sectionStack.pop();
						}

						return messages;
					});
				}
			}
		);

		return await Promise.all([
			unlisten1,
			unlisten2,
			unlisten3,
			unlisten4,
			unlisten5,
			unlisten6,
			unlisten7,
		]);
	});

	onCleanup(() => {
		if (eventUnlistens() != undefined) {
			for (let unlisten of eventUnlistens()!) {
				unlisten();
			}
		}
	});

	// Toast on first launch to install default plugins
	onMount(async () => {
		try {
			let isFirstLaunch = await invoke("get_is_first_launch");
			if (isFirstLaunch) {
				let message = (
					<div class="cont col">
						<div class="cont">
							Would you like to install the default plugins?
						</div>
						<div class="cont">
							<button
								style="border: 0.15rem solid var(--bg3)"
								onclick={(e) => {
									removeThisToast(e.target);
									invoke("install_default_plugins").then(
										() => {
											successToast("Default plugins installed");
										},
										(e) => {
											errorToast("Failed to install default plugins: " + e);
										}
									);
								}}
							>
								Yes
							</button>
							<button
								style="border: 0.15rem solid var(--error)"
								onclick={(e) => {
									removeThisToast(e.target);
								}}
							>
								No
							</button>
						</div>
					</div>
				);

				messageToast(message);
			}
		} catch (e) {}
	});

	let selectedTaskData = createMemo(() => {
		if (selectedTask() == undefined) {
			return undefined;
		} else {
			return Object.values(messages())[selectedTask()!]!;
		}
	});

	return (
		<div id="task-indicator" style={`border-color:${getColors(color())[0]}`}>
			<div
				id="task-indicator-preview"
				style={`color:${getColors(color())[1]}`}
				onclick={() => {
					// Cycle through the selected task when clicking
					if (taskCount() > 0) {
						if (taskCount() == 1 && selectedTask() != undefined) {
							setSelectedTask(undefined);
							return;
						} else if (selectedTask() == taskCount() - 1) {
							setSelectedTask(undefined);
							return;
						}

						let index = selectedTask() == undefined ? 0 : selectedTask()! + 1;
						console.log(index);
						setSelectedTask(index);
					}
				}}
			>
				<Show
					when={taskCount() > 0}
					fallback={
						<div class="cont">
							<div
								id="task-indicator-dot"
								style={`background-color:${getColors(color())[0]}`}
							></div>
						</div>
					}
				>
					<Show when={taskCount() >= 1}>
						<div
							class="cont rotating"
							id="task-indicator-spinner"
							style={`color:${getColors(color())[0]}`}
						>
							<Spinner />
						</div>
					</Show>
				</Show>
				<div class="cont">
					<Show
						when={taskCount() == 1}
						fallback={`${taskCount()} ${
							taskCount() == 1 ? "task" : "tasks"
						} running`}
					>
						{taskName()}
					</Show>
				</div>
			</div>
			<Show when={selectedTask() != undefined}>
				<div
					class="cont col"
					id="task-indicator-popup"
					style={`border-color:${
						getColors(getTaskColor(selectedTaskData()!.id))[0]
					}`}
					onclick={() => setSelectedTask(undefined)}
				>
					<div
						class="cont bold"
						style={`color:${
							getColors(getTaskColor(selectedTaskData()!.id))[1]
						}`}
					>
						{getTaskDisplayName(selectedTaskData()!.id)}
					</div>
					<div class="cont col" id="task-indicator-messages">
						<For each={selectedTaskData()!.sectionStack}>
							{(header) => (
								<Message
									data={{ message: header, messageType: MessageType.Header }}
								/>
							)}
						</For>
						<Show when={selectedTaskData()!.processName != undefined}>
							<Message
								data={{
									message: selectedTaskData()!.processName!,
									messageType: MessageType.StartProcess,
								}}
							/>
						</Show>
					</div>
					<Show when={isTaskKillable(selectedTaskData()!.id)}>
						<div class="cont" id="task-indicator-popup-cancel">
							<IconButton
								icon={Delete}
								size="1.2rem"
								color="var(--errorbg)"
								border="var(--error)"
								selectedColor=""
								selected={false}
								onClick={async (e) => {
									e.preventDefault();
									e.stopPropagation();

									invoke("cancel_task", { task: selectedTaskData()!.id });
								}}
							/>
						</div>
					</Show>
				</div>
			</Show>
		</div>
	);
}

export interface TaskIndicatorProps {}

function Message(props: MessageProps) {
	return (
		<div class="cont start task-indicator-message">
			<Switch>
				<Match when={props.data.messageType == MessageType.Header}>
					<div class="bold">{props.data.message}</div>
				</Match>
				<Match when={props.data.messageType == MessageType.StartProcess}>
					<div class="task-indicator-start-process">
						<div class="cont rotating" id="task-indicator-spinner">
							<Spinner />
						</div>
						<div>{props.data.message}</div>
					</div>
				</Match>
			</Switch>
		</div>
	);
}

interface MessageProps {
	data: MessageData;
}

type MessageData = {
	message: string;
	messageType: MessageType;
};

type TaskMap = {
	[task: string]: Task | undefined;
};

type Task = {
	id: string;
	messages: MessageData[];
	sectionStack: string[];
	processName: string | undefined;
	nextMessageIsProcess: boolean;
	nextMessageIsSection: boolean;
};

export interface MessageEvent {
	message: string;
	type: MessageType;
	task?: string;
}

enum MessageType {
	Simple = "simple",
	Header = "header",
	StartProcess = "start_process",
	Warning = "warning",
	Error = "error",
}

function getTaskDisplayName(task: string) {
	if (task == "get_plugins") {
		return "Getting plugins";
	} else if (task == "update_instance") {
		return "Updating instance";
	} else if (task == "update_instance_packages") {
		return "Updating packages";
	} else if (task.startsWith("launch_instance")) {
		return "Launching";
	} else if (task == "search_packages") {
		return "Searching packages";
	} else if (task == "load_packages") {
		return "Loading packages";
	} else if (task == "sync_packages") {
		return "Syncing packages";
	} else if (task == "login_user") {
		return "Logging in";
	} else if (task == "install_plugins") {
		return "Installing plugins";
	} else if (task == "update_versions") {
		return "Updating versions";
	}
	return beautifyString(task);
}

function getTaskColor(task: string) {
	if (task == "get_plugins" || task == "install_plugins") {
		return "plugin";
	} else if (task.startsWith("launch_instance")) {
		return "instance";
	} else if (
		task == "update_instance" ||
		task == "login_user" ||
		task == "update_versions"
	) {
		return "profile";
	} else if (
		task == "search_packages" ||
		task == "load_packages" ||
		task == "sync_packages" ||
		task == "update_instance_packages"
	) {
		return "package";
	}

	return "running";
}

function isTaskKillable(task: string) {
	return (
		task == "update_instance" ||
		task == "update_instance_packages" ||
		task.startsWith("launch_instance")
	);
}

type Color =
	| "disabled"
	| "running"
	| "instance"
	| "profile"
	| "package"
	| "plugin";

// Gets the border and text colors of a color preset
function getColors(color: Color) {
	if (color == "running") {
		return ["lightgray", "lightgray"];
	} else if (color == "instance") {
		return ["var(--instance)", "var(--instance)"];
	} else if (color == "profile") {
		return ["var(--profile)", "var(--profile)"];
	} else if (color == "package") {
		return ["var(--package)", "var(--package)"];
	} else if (color == "plugin") {
		return ["var(--plugin)", "var(--pluginfg)"];
	}
	return ["var(--bg3)", "var(--fg3)"];
}
