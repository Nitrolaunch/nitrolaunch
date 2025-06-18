import { PackageVersion } from "../../package";
import Modal from "../dialog/Modal";

export default function PackageVersionInfo(props: PackageVersionInfoProps) {
	return (
		<Modal width="50rem" visible={props.visible} onClose={props.onClose}>
			<div id="package-version-info">
				
			</div>
		</Modal>
	);
}

export interface PackageVersionInfoProps {
	visible: boolean;
	version: PackageVersion;
	onClose: () => void;
	onInstall: (version: string) => void;
}
