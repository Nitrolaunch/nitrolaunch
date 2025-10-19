import { JSX, Show } from "solid-js";
import Icon, { HasWidthHeight } from "../Icon";
import "./IconAndText.css";
import Tip, { TipSide } from "../dialog/Tip";

export default function IconAndText(props: IconAndTextProps) {
	let size = props.size == undefined ? "1rem" : props.size;

	return (
		<div
			class={`icon-and-text ${props.bold == true ? "bold" : ""} ${props.centered == true ? "center" : ""}`}
			style={`${props.color == undefined ? "" : `color:${props.color}`}`}
		>
			<div class={`cont icon-and-text-icon ${props.centered == true ? "float" : ""}`} onclick={props.onIconClick}>
				<Show when={props.iconTip != undefined}>
					<Tip tip={props.iconTip} side={props.iconTipSide == undefined ? "top" : props.iconTipSide} cont>
						<Icon icon={props.icon} size={size} />
					</Tip>
				</Show>
				<Show when={props.iconTip == undefined}>
					<Icon icon={props.icon} size={size} />
				</Show>
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
	size?: string;
	color?: string;
	bold?: boolean;
	centered?: boolean;
	iconTip?: string;
	iconTipSide?: TipSide;
	onIconClick?: () => void;
}
