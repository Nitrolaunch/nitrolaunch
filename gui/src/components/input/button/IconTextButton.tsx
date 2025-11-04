import { createSignal, JSXElement, Show } from "solid-js";
import Icon, { HasWidthHeight } from "../../Icon";
import "./IconTextButton.css";

export default function IconTextButton(props: IconTextButtonProps) {
	let [isHovered, setIsHovered] = createSignal(false);

	let color = () => props.color == undefined ? isHovered() ? "var(--bg4)" : "var(--bg3)" : props.color;
	let bgColor = () => props.bgColor == undefined ? isHovered() ? "var(--bg3)" : "var(--bg2)" : props.bgColor;
	let textColor = () => props.color == undefined ? "var(--fg)" : props.color;

	const colorStyle = () => `background-color:${bgColor()};border-color:${color()};color:${textColor()}`;

	let shadow = props.shadow == undefined ? true : props.shadow;

	return (
		<button
			class={`${shadow ? "shadow" : ""} bubble-hover icon-text-button bold`}
			style={`${colorStyle()};${props.style == undefined ? "" : props.style}`}
			onClick={props.onClick}
			onmouseenter={() => setIsHovered(true)}
			onmouseleave={() => setIsHovered(false)}
		>
			<Show when={props.icon != undefined}>
				<div
					class={`icon-text-button-icon center ${props.animate == true ? "rotating" : ""
						}`}
				>
					<Icon icon={props.icon!} size={`calc(${props.size} * 0.7)`} />
				</div>
			</Show>
			<div class="icon-text-button-text">{props.text}</div>
		</button>
	);
}

export interface IconTextButtonProps {
	icon?: (props: HasWidthHeight) => JSXElement;
	size: string;
	text: string;
	color?: string;
	bgColor?: string;
	shadow?: boolean;
	animate?: boolean;
	style?: string;
	onClick: () => void;
}
