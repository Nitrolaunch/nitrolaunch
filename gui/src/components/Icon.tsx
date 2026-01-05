import { JSXElement } from "solid-js";

export default function Icon(props: IconProps) {
	return (
		<props.icon
			width={props.size}
			height={props.size}
			viewBox={`0 0 16 16`}
			// style={`${props.color == undefined ? "" : `color:${props.color}`}`}
			{...props}
		/>
	);
}

export interface IconProps {
	icon: (props: HasWidthHeight) => JSXElement;
	size: string;
	// color?: string;
	[prop: string]: any;
}

export function HTMLIcon(html: string) {
	return (props: HasWidthHeight) => {
		let attrs = `width=${props.width} height=${props.height} viewBox=${props.viewBox}`;
		let html2 = html.replace("<svg", `<svg ${attrs} `);
		return <div class="cont" innerHTML={html2}></div>;
	};
}

export interface HasWidthHeight {
	width?: string;
	height?: string;
	viewBox?: string;
	[prop: string]: any;
}
