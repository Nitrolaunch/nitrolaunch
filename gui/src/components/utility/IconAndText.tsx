import { JSX } from "solid-js";
import Icon, { HasWidthHeight } from "../Icon";
import "./IconAndText.css";

export default function IconAndText(props: IconAndTextProps) {
	return (
		<div
			class={`icon-and-text ${props.bold == true ? "bold" : ""} ${props.centered == true ? "center" : ""}`}
			style={`${props.color == undefined ? "" : `color:${props.color}`}`}
		>
			<div class={`cont icon-and-text-icon ${props.centered == true ? "float" : ""}`}>
				<Icon icon={props.icon} size="1rem" />
			</div>
			<div class={`cont icon-and-text-text ${props.centered == true ? "center" : "start"}`}>
				{props.text}
			</div>
		</div>
	);
}

export interface IconAndTextProps {
	icon: (props: HasWidthHeight) => JSX.Element;
	text: JSX.Element;
	color?: string;
	bold?: boolean;
	centered?: boolean;
}
