import { JSX } from "solid-js";
import Icon, { HasWidthHeight } from "../Icon";
import "./IconAndText.css";

export default function IconAndText(props: IconAndTextProps) {
	return (
		<div
			class={`icon-and-text ${props.bold == true ? "bold" : ""}`}
			style={`${props.color == undefined ? "" : `color:${props.color}`}`}
		>
			<div class="cont">
				<Icon icon={props.icon} size="1rem" />
			</div>
			<div class={`cont ${props.centered == true ? "" : "start"}`}>
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
