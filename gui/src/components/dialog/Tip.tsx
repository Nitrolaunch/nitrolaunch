import { JSX } from "solid-js";
import "./Tip.css";

export default function Tip(props: TipProps) {
	let fullwidth = props.fullwidth == undefined ? false : props.fullwidth;

	return (
		<div
			class={`${props.cont == true ? "cont" : ""}`}
			style={`${fullwidth ? "width:100%" : ""}`}
			data-tip={props.tip}
		>
			{props.children}
		</div>
	);
}

export interface TipProps {
	children: JSX.Element;
	tip: string;
	side?: TipSide;
	fullwidth?: boolean;
	zIndex?: string;
	cont?: boolean;
}

export type TipSide = "top" | "bottom" | "right" | "left";
