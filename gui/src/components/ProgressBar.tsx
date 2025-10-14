import "./ProgressBar.css";

export default function ProgressBar(props: ProgressBarProps) {
	return <div class="cont fullwidth progress-bar" style={`border-color: ${props.color}`}>
		<div class="cont progress-bar-value" style={`background-color: ${props.color};width:${props.value * 100}%`}></div>
	</div>
}

export interface ProgressBarProps {
	// Percent completion of the bar from 0 to 1
	value: number;
	// Color of the bar
	color: string;
}
