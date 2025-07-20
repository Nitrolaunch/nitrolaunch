import { createResource, For, Match, Suspense, Switch } from "solid-js";
import Modal from "../dialog/Modal";
import { invoke } from "@tauri-apps/api";
import { InstanceOrProfile } from "../../types";
import { beautifyString } from "../../utils";
import "./ProfileDeletePrompt.css";
import IconTextButton from "../input/IconTextButton";
import { Delete } from "../../icons";
import { errorToast, successToast } from "../dialog/Toasts";

export default function ProfileDeletePrompt(props: ProfileDeletePromptProps) {
	let [profileUsers, _] = createResource(
		() => props.profile,
		async () => {
			if (props.profile == undefined) {
				return undefined;
			}

			return (await invoke("get_profile_users", {
				profile: props.profile,
			})) as [string, InstanceOrProfile][];
		}
	);

	return (
		<Modal visible={props.visible} onClose={props.onClose} width="25rem">
			<div class="cont col" style="padding:2rem">
				<Suspense>
					<Switch>
						<Match
							when={profileUsers() == undefined || profileUsers()!.length > 0}
						>
							<h3>
								Cannot delete this profile as there are instances or profiles
								using it
							</h3>
							<For each={profileUsers()!}>
								{(item) => (
									<div
										class="profile-delete-prompt-entry"
										onclick={() => {
											props.onClose();
											window.location.href = `/${item[1]}_config/${item[0]}`;
										}}
									>
										<div
											class="cont profile-delete-prompt-entry-type"
											style={`color:var(--${item[1]})`}
										>
											{beautifyString(item[1])}
										</div>
										<div class="cont start profile-delete-prompt-entry-id">
											{item[0]}
										</div>
									</div>
								)}
							</For>
						</Match>
						<Match
							when={profileUsers() != undefined && profileUsers()!.length == 0}
						>
							<h3>Are you sure you want to delete this profile?</h3>
							<div></div>
							<div></div>
							<div class="cont">
								<button
									onclick={props.onClose}
									style="border-color:var(--instance)"
								>
									Cancel
								</button>
								<IconTextButton
									icon={Delete}
									size="1rem"
									text="Delete profile"
									color="var(--errorbg)"
									selectedColor="var(--error)"
									selectedBg="var(--errorbg)"
									selected={true}
									onClick={async () => {
										try {
											await invoke("delete_profile", {
												profile: props.profile,
											});
											successToast("Profile deleted");
											props.onClose();
											window.location.href = "/";
										} catch (e) {
											errorToast("Failed to delete profile: " + e);
											props.onClose();
										}
									}}
								/>
							</div>
						</Match>
					</Switch>
				</Suspense>
			</div>
		</Modal>
	);
}

export interface ProfileDeletePromptProps {
	profile?: string;
	visible: boolean;
	onClose: () => void;
}
