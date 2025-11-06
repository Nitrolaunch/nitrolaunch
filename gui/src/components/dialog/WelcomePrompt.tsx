import { createSignal, Match, Show, Switch } from "solid-js";
import ModalBase from "./ModalBase";

import "./WelcomePrompt.css";
import IconTextButton from "../input/button/IconTextButton";
import { AngleLeft, AngleRight, Check, Delete } from "../../icons";
import { invoke } from "@tauri-apps/api/core";
import { errorToast, successToast } from "./Toasts";
import Icon from "../Icon";
import { MigratePromptContents } from "../instance/MigratePrompt";

export default function WelcomePrompt(props: WelcomePromptProps) {
	let [tab, setTab] = createSignal(0);

	return <ModalBase visible={props.visible} onClose={() => { }} width="30rem">
		<div id="welcome-prompt">
			<div id="welcome-prompt-tabs" style={`grid-template-columns:repeat(3,minmax(0,1fr))`}>
				<div
					class={`cont welcome-prompt-tab ${tab() == 0 ? "selected" : ""} first`}
					style="border-top-left-radius:var(--round2)"
				>
					<Show when={tab() > 0}>
						<Icon icon={Check} size="1rem" />
					</Show>
					1. Plugins
				</div>
				<div
					class={`cont welcome-prompt-tab ${tab() == 1 ? "selected" : ""}`}
				>
					<Show when={tab() > 1}>
						<Icon icon={Check} size="1rem" />
					</Show>
					2. Migrate
				</div>
				<div
					class={`cont welcome-prompt-tab ${tab() == 2 ? "selected" : ""} last`}
					style="border-top-right-radius:var(--round2)"
				>
					3. Welcome!
				</div>
			</div>
			<div class="cont col" id="welcome-prompt-contents">
				<Show when={tab() == 0}>
					<div class="cont col" style="width:80%;text-align:center">
						<span class="bold">
							Would you like to install recommended plugins?
						</span>
						<span style="color:var(--fg2)">
							This includes features like modloader installation, Modrinth integration,
							and importing from other launchers.
						</span>
					</div>
					<br />
					<div class="cont">
						<IconTextButton icon={Delete} size="1.5rem" color="var(--error)" bgColor="var(--errorbg)" text="No" onClick={() => {
							setTab(1);
						}} />
						<IconTextButton icon={Check} size="1.5rem" color="var(--instance)" bgColor="var(--instancebg)" text="Yes" onClick={() => {
							invoke("install_default_plugins").then(
								() => {
									successToast("Default plugins installed");
									setTab(1);
								},
								(e) => {
									errorToast("Failed to install default plugins: " + e);
								}
							);
						}} />
					</div>
				</Show>
				<Show when={tab() == 1}>
					<MigratePromptContents visible onClose={() => setTab(2)} />
				</Show>
				<Show when={tab() == 2}>
					<span class="bold">
						Welcome to Nitrolaunch!
					</span>
				</Show>
			</div>
			<div id="welcome-prompt-navigation" class="split">
				<div class="cont start" style="padding-left: 1rem">
					<Show when={tab() != 0}>
						<IconTextButton icon={AngleLeft} size="1.5rem" text="Back" onClick={() => {
							setTab(tab() - 1);
						}} />
					</Show>
				</div>
				<div class="cont end" style="padding-right: 1rem">
					<Switch>
						<Match when={tab() != 2}>
							<IconTextButton icon={AngleRight} size="1.5rem" text="Next" onClick={() => {
								setTab(tab() + 1);
							}} />
						</Match>
						<Match when={tab() == 2}>
							<IconTextButton icon={Check} size="1.5rem" text="Done!" onClick={() => {
								props.onClose();
							}} />
						</Match>
					</Switch>
				</div>
			</div>
		</div>
	</ModalBase>;
}

export interface WelcomePromptProps {
	visible: boolean;
	onClose: () => void;
}
