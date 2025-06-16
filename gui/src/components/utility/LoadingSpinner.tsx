import { Spinner } from "../../icons";
import Icon from "../Icon";

export default function LoadingSpinner(props: LoadingSpinnerProps) {
	return (
		<div class="loading-spinner rotating" style="color: var(--fg3)">
			<Icon icon={Spinner} size={props.size} />
		</div>
	);
}

export interface LoadingSpinnerProps {
	size: string;
}
