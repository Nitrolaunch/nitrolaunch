import { createResource, For, Match, Suspense, Switch } from "solid-js";
import { invoke } from "@tauri-apps/api";
import { InstanceOrProfile } from "../../types";
import { beautifyString } from "../../utils";
import "./ProfileDeletePrompt.css";
import { Delete, Trash } from "../../icons";
import { errorToast, successToast } from "../dialog/Toasts";
import { useNavigate } from "@solidjs/router";
import Modal, { ModalButton } from "../dialog/Modal";

export default function ProfileDeletePrompt(props: ProfileDeletePromptProps) {
	let navigate = useNavigate();

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

	let canDelete = () => profileUsers() != undefined && profileUsers()!.length == 0;

	let buttons = () => {
		let out: ModalButton[] = [{
			text: "Cancel",
			icon: Delete,
			color: "var(--instance)",
			bgColor: "var(--instancebg)",
			onClick: props.onClose,
		}];

		if (canDelete()) {
			out.push(
				{
					text: "Delete profile",
					icon: Trash,
					color: "var(--fg3)",
					onClick: async () => {
						try {
							await invoke("delete_profile", {
								profile: props.profile,
							});
							successToast("Profile deleted");
							props.onClose();
							navigate("/");
						} catch (e) {
							errorToast("Failed to delete profile: " + e);
							props.onClose();
						}
					}
				}
			);
		}

		return out;
	}

	return (
		<Modal
			visible={props.visible}
			onClose={props.onClose}
			title={`Delete Profile '${props.profile}'`}
			titleIcon={Trash}
			buttons={buttons()}
		>
			<Suspense>
				<Switch>
					<Match
						when={!canDelete()}
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
										navigate(`/${item[1]}_config/${item[0]}`);
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
						when={canDelete()}
					>
						<h3>Are you sure you want to delete this profile?</h3>
					</Match>
				</Switch>
			</Suspense>
		</Modal>
	);
}

export interface ProfileDeletePromptProps {
	profile?: string;
	visible: boolean;
	onClose: () => void;
}
