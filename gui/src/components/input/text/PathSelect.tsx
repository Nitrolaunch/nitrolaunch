import { open } from "@tauri-apps/plugin-dialog";
import { Folder } from "../../../icons";
import IconTextButton from "../button/IconTextButton";
import "./PathSelect.css";

export default function PathSelect(props: PathSelectProps) {
	return <div class="path-select">
		<div class="cont path-select-path">
			{props.path == undefined || props.path == "" ? "No file selected" : props.path}
		</div>
		<IconTextButton icon={Folder} size="1rem" text="Select" onClick={async () => {
			props.setPath(await open() as string);
		}} />
	</div>;
}

export interface PathSelectProps {
	path?: string;
	setPath: (path: string) => void;
}
