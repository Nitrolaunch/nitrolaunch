import { Spinner } from "../../icons";
import Icon from "../Icon";

export default function LoadingSpinner(props: LoadingSpinnerProps) {
	return (
		<div
			class="loading-spinner rotating"
			style={`color: var(--bg4);width:${props.size};height:${props.size}`}
		>
			<Icon icon={Spinner} size={props.size} />
		</div>
	);
}

export interface LoadingSpinnerProps {
	size: string;
}
