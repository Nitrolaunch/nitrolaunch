import { JSX, Show } from "solid-js";

// A variant of the SolidJS Show component that uses a display:none attribute instead to not have to reload the whole contents.
// Note that this won't help with DOM performance, just if you have components with resources that you don't want to refetch
export default function DisplayShow(props: DisplayShowProps) {
	let when = () => (props.when == undefined ? false : props.when);
	let style = () => (props.style == undefined ? "" : props.style);

	return (
		<div class="display-show-container" style={style()}>
			<div
				class="display-show"
				style={`${when() ? "" : "display:none"};${style}`}
			>
				{props.children}
			</div>
			<Show when={!when() && props.fallback != undefined}>
				{props.fallback}
			</Show>
		</div>
	);
}

export interface DisplayShowProps {
	children: JSX.Element;
	when?: boolean;
	fallback?: JSX.Element;
	style?: string;
}
