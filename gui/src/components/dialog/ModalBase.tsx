import { JSX, Show } from "solid-js";
import PageBlock from "../PageBlock";
import "./ModalBase.css";

// Simple modal with no contents
export default function ModalBase(props: ModalBaseProps) {
	return (
		<Show when={props.visible}>
			<PageBlock onClick={() => props.onClose(false)} />
			<div class="cont modal-container">
				<div
					class="cont modal-behind"
					onclick={() => props.onClose(false)}
				></div>
				<div class="cont modal fade-in-fast pop-in-fast" style={`width:${props.width}`}>
					{props.children}
				</div>
			</div>
		</Show>
	);
}

export interface ModalBaseProps {
	children: JSX.Element;
	visible: boolean;
	width: string;
	onClose: (visible: boolean) => void;
}
