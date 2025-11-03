import { createSignal, For, JSX, Show } from "solid-js";
import ModalBase from "./ModalBase";
import "./Modal.css";
import Icon, { HasWidthHeight } from "../Icon";
import IconButton from "../input/button/IconButton";
import { Delete } from "../../icons";

// Modal with a title box and buttons at the bottom
export default function Modal(props: ModalProps) {
	let width = props.width == undefined ? "30rem" : props.width;
	let height = props.height == undefined ? "20rem" : props.height;

	let buttons = <For each={props.buttons}>
		{(button, i) => {
			let [isHovered, setIsHovered] = createSignal(false);

			let textColor = () => {
				if (button.color == undefined) {
					return "var(--fg2)";
				} else {
					return button.color;
				}
			};

			let borderColor = () => {
				if (button.color == undefined) {
					if (isHovered()) {
						return "var(--bg4)";
					} else {
						return "var(--bg3)";
					}
				} else {
					return button.color;
				}
			};

			let bgColor = () => {
				if (button.bgColor == undefined) {
					if (isHovered()) {
						return "var(--bg3)";
					} else {
						return "var(--bg2)";
					}
				} else {
					return button.bgColor;
				}
			};

			return <div
				class={`cont modal-button ${i() == 0 ? "first" : ""} ${i() == props.buttons.length - 1 ? "last" : ""}`}
				onmouseenter={() => setIsHovered(true)}
				onmouseleave={() => setIsHovered(false)}
				style={`color:${textColor()};border-top-color:${borderColor()};background-color:${bgColor()}`}
				onclick={button.onClick}
			>
				<Icon icon={button.icon} size="1.2rem" />
				{button.text}
			</div>;
		}}
	</For>

	return <ModalBase
		visible={props.visible}
		width={width}
		onClose={props.onClose == undefined ? () => { } : props.onClose}
	>
		<div class="modal-contents">
			<div class="cont modal-header">
				<Show when={props.titleIcon != undefined}>
					<Icon icon={props.titleIcon!} size="1rem" />
				</Show>
				{props.title}
				<Show when={props.onClose != undefined}>
					<div class="cont modal-x">
						<IconButton
							icon={Delete}
							size="1.35rem"
							color="var(--bg2)"
							iconColor="var(--fg3)"
							hoverBackground="var(--bg3)"
							onClick={() => props.onClose!(false)}
						/>
					</div>
				</Show>
			</div>
			<div class="cont col start modal-body" style={`height:${height}`}>
				{props.children}
			</div>
			<Show when={props.buttons.length > 0}>
				<div class="modal-buttons" style={`grid-template-columns:repeat(${props.buttons.length}, 1fr)`}>
					{buttons}
				</div>
			</Show>
		</div>
	</ModalBase>
}

export interface ModalProps {
	children: JSX.Element;
	visible: boolean;
	width?: string;
	height?: string;
	onClose?: (visible: boolean) => void;
	title: JSX.Element;
	titleIcon?: (props: HasWidthHeight) => JSX.Element;
	buttons: ModalButton[];
}

export interface ModalButton {
	text: string;
	icon: (props: HasWidthHeight) => JSX.Element;
	color?: string;
	bgColor?: string;
	onClick: () => void;
}
