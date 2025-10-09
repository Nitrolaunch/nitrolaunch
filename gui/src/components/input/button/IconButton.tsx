import { createSignal, JSXElement } from "solid-js";
import "./IconButton.css";
import Icon, { HasWidthHeight } from "../../Icon";

export default function IconButton(props: IconButtonProps) {
	let [isHovered, setIsHovered] = createSignal(false);

	let backgroundColor = () => {
		if (props.selected == true && props.selectedColor != undefined) {
			return props.selectedColor;
		} else if (isHovered() && props.hoverBackground != undefined) {
			return props.hoverBackground;
		} else {
			return props.color;
		}
	};

	let border = () => {
		if (props.hoverBorder != undefined && isHovered()) {
			return `border-color: ${props.hoverBorder}`;
		} else if (props.border == undefined) {
			return `border-color: ${backgroundColor()}`;
		} else {
			return `border-color: ${props.border}`;
		}
	};

	let colorStyle = () => `background-color:${backgroundColor()};${border()}`;

	let iconColorStyle =
		props.iconColor == undefined ? "" : `color:${props.iconColor}`;

	let isCircle = props.circle == undefined ? false : props.circle;

	return (
		<div
			class={`cont icon-button ${isCircle ? "circle" : ""} ${props.shadow == true ? "input-shadow" : ""
				} bubble-hover`}
			style={`${colorStyle()};width:${props.size};height:${props.size
				};${iconColorStyle}`}
			onClick={props.onClick}
			onmouseenter={() => setIsHovered(true)}
			onmouseleave={() => setIsHovered(false)}
		>
			<Icon icon={props.icon} size={`calc(${props.size} * 0.7)`} />
		</div>
	);
}

export interface IconButtonProps {
	icon: (props: HasWidthHeight) => JSXElement;
	size: string;
	color: string;
	iconColor?: string;
	selectedColor?: string;
	border?: string;
	hoverBorder?: string;
	hoverBackground?: string;
	circle?: boolean;
	shadow?: boolean;
	selected?: boolean;
	onClick: (e: Event) => void;
}
