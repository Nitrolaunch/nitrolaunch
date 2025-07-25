import {
	createSignal,
	createUniqueId,
	For,
	JSX,
	onMount,
	Show,
} from "solid-js";
import "./Toasts.css";
import Icon from "../Icon";
import { Check, Delete, Error, Warning } from "../../icons";

export default function Toasts() {
	let [toasts, setToasts] = createSignal<ToastProps[]>([]);
	// Trick to re-render since just updating the toasts signal doesnt work on its own
	let [visible, setVisible] = createSignal(true);

	// Removes a toast at an index and updates the list
	function removeToast(index: number) {
		setToasts((toasts) => {
			toasts.splice(index, 1);
			return toasts;
		});
		setVisible(false);
		setVisible(true);
	}

	onMount(() => {
		// Global function for adding toasts
		let win = window as any;
		win.__createToast = (props: ToastProps) => {
			setToasts((toasts) => {
				if (toasts.length >= 4) {
					toasts.splice(0, 1);
				}
				toasts.push(props);
				return toasts;
			});
			setVisible(false);
			setVisible(true);
		};

		win.__toastRemoveFunctions = {};

		win.__removeToast = (id: string) => {
			if (win.__toastRemoveFunctions[id] != undefined) {
				win.__toastRemoveFunctions[id]();
			}
		};

		// Periodically remove toasts that have an age set
		clearInterval(win.toastIntervalId);
		win.toastIntervalId = setInterval(() => {
			for (let i = 0; i < toasts().length; i++) {
				let toast = toasts()[i];
				if (toast.age != undefined) {
					if (toast.age <= 0) {
						removeToast(i);
					} else {
						toast.age -= 0.1;
					}
				}
			}
		}, 100);
	});

	return (
		<Show when={visible()}>
			<div id="toasts" class="cont col">
				<For each={toasts()}>
					{(props, i) => (
						<Toast
							{...props}
							onRemove={() => {
								removeToast(i());
							}}
						/>
					)}
				</For>
			</div>
		</Show>
	);
}

function Toast(props: ToastProps) {
	let [isHovered, setIsHovered] = createSignal(false);

	let Icon2 = () => {
		if (props.type == "message") {
			return <div></div>;
		} else if (props.type == "success") {
			return <Check />;
		} else if (props.type == "warning") {
			return <Warning />;
		} else if (props.type == "error") {
			return <Error />;
		} else {
			return <div></div>;
		}
	};

	let id = createUniqueId();

	let win = window as any;
	win.__toastRemoveFunctions[id] = props.onRemove;

	return (
		<div
			class={`cont toast ${props.type}`}
			onmouseenter={() => setIsHovered(true)}
			onmouseleave={() => setIsHovered(false)}
			data-id={id}
		>
			<div class="cont toast-icon">
				<Icon2 />
			</div>
			<div class="toast-message">{props.message}</div>
			<Show when={isHovered()}>
				<div class="toast-x" onclick={props.onRemove}>
					<Icon class="toast-x" icon={Delete} size="1rem" />
				</div>
			</Show>
		</div>
	);
}

interface ToastProps {
	message: JSX.Element;
	type: ToastType;
	age?: number;
	maxAge?: number;
	onRemove: () => void;
}

type ToastType = "message" | "success" | "warning" | "error";

export function messageToast(message: JSX.Element) {
	(window as any).__createToast({ message: message, type: "message" });
}

export function successToast(message: JSX.Element) {
	(window as any).__createToast({
		message: message,
		type: "success",
		age: 3,
		maxAge: 3,
	});
}

export function warningToast(message: JSX.Element) {
	(window as any).__createToast({
		message: message,
		type: "warning",
		age: 7,
		maxAge: 7,
	});
}

export function errorToast(message: JSX.Element) {
	(window as any).__createToast({
		message: message,
		type: "error",
		age: 9,
		maxAge: 9,
	});
	console.error("Error: " + message);
}

// Closes this toast. Called from buttons in a toast message
export function removeThisToast(elem: Element) {
	let id = getThisToastId(elem);
	if (id != undefined) {
		(window as any).__removeToast(id);
	}
}

// Given an element, gets the unique ID of the toast above it
function getThisToastId(elem: Element) {
	if (elem.classList.contains("toast")) {
		return (elem as any).dataset.id as string;
	}

	let parent = elem.parentElement;
	if (parent == null) {
		return undefined;
	} else {
		return getThisToastId(parent);
	}
}
