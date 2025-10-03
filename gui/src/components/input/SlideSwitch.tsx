import "./SlideSwitch.css";

export default function SlideSwitch(props: SlideSwitchProps) {
	let color = () => props.enabled ? props.enabledColor : props.disabledColor;

	let handleTransform = () => props.enabled ? .55 : -.55;

	return <div class="cont bubble-hover slide-switch" style={`color:${color()};border-color:${color()}`} onclick={props.onToggle}>
		<div class="cont slide-switch-handle" style={`background-color:${color()};transform:translateX(${handleTransform()}rem)`}></div>
	</div >;
}

export interface SlideSwitchProps {
	enabled: boolean;
	onToggle: () => void;
	disabledColor: string;
	enabledColor: string;
}
