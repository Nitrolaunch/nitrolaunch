import "./SlideSwitch.css";

export default function SlideSwitch(props: SlideSwitchProps) {
	let color = () => (props.enabled ? props.enabledColor : props.disabledColor);
	let bgColor = () =>
		props.enabledBg != undefined && props.enabled
			? `background-color:${props.enabledBg}`
			: "";

	let handleTransform = () => (props.enabled ? 0.5 : -0.5);

	return (
		<div
			class="cont bubble-hover slide-switch"
			style={`color:${color()};border-color:${color()};${bgColor()}`}
			onclick={props.onToggle}
		>
			<div
				class="cont slide-switch-handle"
				style={`background-color:${color()};transform:translateX(${handleTransform()}rem)`}
			></div>
		</div>
	);
}

export interface SlideSwitchProps {
	enabled: boolean;
	onToggle: () => void;
	disabledColor: string;
	enabledColor: string;
	enabledBg?: string;
}
