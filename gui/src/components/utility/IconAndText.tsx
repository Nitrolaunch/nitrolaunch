import { JSX, Show } from "solid-js";
import Icon, { HasWidthHeight } from "../Icon";
import "./IconAndText.css";
import Tip, { TipSide } from "../dialog/Tip";

export default function IconAndText(props: IconAndTextProps) {
	return (
		<div
			class={`icon-and-text ${props.bold == true ? "bold" : ""} ${props.centered == true ? "center" : ""}`}
			style={`${props.color == undefined ? "" : `color:${props.color}`}`}
		>
			<div class={`cont icon-and-text-icon ${props.centered == true ? "float" : ""}`} onclick={props.onIconClick}>
				<Show when={props.iconTip != undefined}>
					<Tip tip={props.iconTip} side={props.iconTipSide == undefined ? "top" : props.iconTipSide} cont>
						<Icon icon={props.icon} size="1rem" />
					</Tip>
				</Show>
				<Show when={props.iconTip == undefined}>
					<Icon icon={props.icon} size="1rem" />
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
	color?: string;
	bold?: boolean;
	centered?: boolean;
	iconTip?: string;
	iconTipSide?: TipSide;
	onIconClick?: () => void;
}
