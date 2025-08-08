import { JSX } from "solid-js";
import Icon, { HasWidthHeight } from "../Icon";
import "./IconAndText.css";

export default function IconAndText(props: IconAndTextProps) {
	return (
		<div class="icon-and-text">
			<div class="cont">
				<Icon icon={props.icon} size="1rem" />
			</div>
			<div class="cont start">{props.text}</div>
		</div>
	);
}

export interface IconAndTextProps {
	icon: (props: HasWidthHeight) => JSX.Element;
	text: JSX.Element;
}
