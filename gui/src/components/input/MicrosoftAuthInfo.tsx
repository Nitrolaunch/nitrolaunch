import { createSignal } from "solid-js";
import { AuthDisplayEvent } from "../../types";
import "./MicrosoftAuthInfo.css";
import IconTextButton from "./IconTextButton";
import { clipboard } from "@tauri-apps/api";
import { Check, Copy, Globe } from "../../icons";
import { WebviewWindow } from "@tauri-apps/api/window";
import Modal from "../dialog/Modal";

export default function MicrosoftAuthInfo(props: MicrosoftAuthInfoProps) {
	return (
		<Modal visible={props.visible} width="20rem" onClose={() => {}}>
			<div class="cont col ms-auth-info">
				Copy this code:
				<CopyCodeButton code={props.event.device_code} />
				Then paste it into the login page:
				<LoginWindowButton url={props.event.url} />
			</div>
		</Modal>
	);
}

function CopyCodeButton(props: CopyCodeButtonProps) {
	const [clicked, setClicked] = createSignal(false);

	return (
		<IconTextButton
			text={clicked() ? "Copied!" : "Click to copy"}
			size="18px"
			icon={clicked() ? Check : Copy}
			color="var(--bg2)"
			selectedColor="var(--accent)"
			selected={clicked()}
			onClick={async () => {
				setClicked(true);
				await clipboard.writeText(props.code);
				setTimeout(() => {
					setClicked(false);
				}, 3000);
			}}
		/>
	);
}

interface CopyCodeButtonProps {
	code: string;
}

function LoginWindowButton(props: LoginWindowButtonProps) {
	const [opening, setOpening] = createSignal(false);

	return (
		<IconTextButton
			text={opening() ? "Opening..." : "Open login page"}
			size="18px"
			icon={Globe}
			color="var(--bg2)"
			selectedColor="var(--accent)"
			selected={opening()}
			onClick={async () => {
				setOpening(true);
				const loginWindow = new WebviewWindow("microsoft_login", {
					url: props.url,
				});
				loginWindow.once("tauri://error", (e) => {
					console.error("Failed to create login window: " + e.payload);
				});
				setTimeout(() => {
					setOpening(false);
				}, 3000);
			}}
		/>
	);
}

interface LoginWindowButtonProps {
	url: string;
}

export interface MicrosoftAuthInfoProps {
	visible: boolean;
	event: AuthDisplayEvent;
	onCancel: () => void;
}
