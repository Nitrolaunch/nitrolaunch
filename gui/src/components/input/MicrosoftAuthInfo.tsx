import { createSignal } from "solid-js";
import { AuthDisplayEvent } from "../../types";
import "./MicrosoftAuthInfo.css";
import IconTextButton from "./button/IconTextButton";
import { Check, Copy, Globe, Lock } from "../../icons";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open } from "@tauri-apps/plugin-shell";
import Modal from "../dialog/Modal";
import * as clipboard from "@tauri-apps/plugin-clipboard-manager"

export default function MicrosoftAuthInfo(props: MicrosoftAuthInfoProps) {
	return (
		<Modal visible={props.visible} onClose={() => { }} title="Microsoft Authentication" titleIcon={Lock} buttons={[]}>
			Copy this code:
			<CopyCodeButton code={props.event.device_code} />
			Then paste it into the login page:
			<LoginWindowButton url={props.event.url} inBrowser={false} />
			If that link doesn't work, trying opening in your browser instead:
			<LoginWindowButton url={props.event.url} inBrowser={true} />
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
			color={clicked() ? "var(--instance)" : "var(--fg)"}
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
			color={opening() ? "var(--template)" : "var(--fg)"}
			onClick={async () => {
				setOpening(true);
				if (props.inBrowser) {
					open(props.url);
				} else {
					const loginWindow = new WebviewWindow("microsoft_login", {
						url: props.url,
						title: "Microsoft Login"
					});
					loginWindow.once("tauri://error", (e) => {
						console.error("Failed to create login window: " + e.payload);
					});
				}
				setTimeout(() => {
					setOpening(false);
				}, 3000);
			}}
		/>
	);
}

interface LoginWindowButtonProps {
	url: string;
	inBrowser: boolean;
}

export interface MicrosoftAuthInfoProps {
	visible: boolean;
	event: AuthDisplayEvent;
	onCancel: () => void;
}
