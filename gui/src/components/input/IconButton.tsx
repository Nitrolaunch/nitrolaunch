import { createSignal, JSXElement } from "solid-js";
import "./IconButton.css";
import Icon, { HasWidthHeight } from "../Icon";

export default function IconButton(props: IconButtonProps) {
	let [isHovered, setIsHovered] = createSignal(false);

	let backgroundColor = () =>
		props.selected ? props.selectedColor : props.color;

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

	return (
		<div
			class="cont icon-button"
			style={`${colorStyle()};width:${props.size};height:${
				props.size
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
	color: string;
	selectedColor: string;
	iconColor?: string;
	border?: string;
	hoverBorder?: string;
	size: string;
	selected: boolean;
	onClick: (e: Event) => void;
}
