import { useParams } from "@solidjs/router";
import { createResource, Match, onMount, Show, Switch } from "solid-js";
import { loadPagePlugins } from "../../plugins";
import { errorToast, successToast } from "../../components/dialog/Toasts";
import LoadingSpinner from "../../components/utility/LoadingSpinner";
import getUserIcon, { beautifyString } from "../../utils";
import { Delete, Lock, LockOpen } from "../../icons";
import "./UserPage.css";
import IconTextButton from "../../components/input/IconTextButton";
import { invoke } from "@tauri-apps/api";
import { UserInfo } from "../../components/user/UserWidget";
import { emit } from "@tauri-apps/api/event";

export default function UserPage() {
	let params = useParams();
	let id = params.userId;

	onMount(() => loadPagePlugins("user", id));

	let [user, userOperations] = createResource(async () => {
		try {
			let [_, users] = (await invoke("get_users")) as [
				string | undefined,
				{ [id: string]: UserInfo }
			];

			return users[id];
		} catch (e) {
			errorToast("Failed to get user: " + e);
			return undefined;
		}
	});

	return (
		<Show
			when={user() != undefined}
			fallback={
				<div class="cont" style="width:100%">
					<LoadingSpinner size="5rem" />
				</div>
			}
		>
			<div class="cont col" style="width:100%">
				<div class="cont col" id="user-container">
					<div class="cont" id="user-header-container">
						<div class="input-shadow" id="user-header">
							<div class="cont start" id="user-icon">
								<img
									id="user-icon-image"
									src={getUserIcon(user()!.uuid)}
									onerror={(e) =>
										((e.target as any).src = getUserIcon(undefined))
									}
								/>
							</div>
							<div id="user-details-container">
								<div class="col" id="user-details">
									<div class="cont" id="user-upper-details">
										<div id="user-name">
											{user()!.username == undefined ? id : user()!.username}
										</div>
										<Show when={user()!.username != undefined}>
											<div id="user-id">{id}</div>
										</Show>
									</div>
									<div class="cont start" id="user-lower-details">
										<div class="cont" id="user-type">
											{beautifyString(user()!.type).toLocaleUpperCase()}
										</div>
									</div>
								</div>
								<div class="cont end" style="margin-right:1rem">
									<Switch>
										<Match when={user()!.username == undefined}>
											<IconTextButton
												icon={LockOpen}
												size="1.2rem"
												text="Log In"
												color="var(--bg2)"
												selected={false}
												selectedColor="var(--instance)"
												onClick={async () => {
													try {
														await invoke("login_user", { user: id });
														successToast("Logged in");
														userOperations.refetch();
														emit("refresh_users");
													} catch (e) {
														errorToast("Failed to log in: " + e);
													}
												}}
												shadow={false}
											/>
										</Match>
										<Match when={user()!.username != undefined}>
											<IconTextButton
												icon={Lock}
												size="1.2rem"
												text="Log Out"
												color="var(--bg2)"
												selected={false}
												selectedColor="var(--instance)"
												onClick={async () => {
													try {
														await invoke("logout_user", { user: id });
														successToast("Logged out");
														userOperations.refetch();
														emit("refresh_users");
													} catch (e) {
														errorToast("Failed to log out: " + e);
													}
												}}
												shadow={false}
											/>
										</Match>
									</Switch>
									<IconTextButton
										icon={Delete}
										size="1.2rem"
										text="Delete"
										color="var(--bg2)"
										selected={true}
										selectedColor="var(--error)"
										selectedBg="var(--errorbg)"
										onClick={async () => {
											try {
												await invoke("remove_user", {
													user: id,
												});
												successToast("User deleted");
												window.location.href = "/";
											} catch (e) {
												errorToast("Failed to delete user: " + e);
											}
										}}
										shadow={false}
									/>
								</div>
							</div>
						</div>
					</div>
					<div id="user-body" class="input-shadow"></div>
				</div>
				<br />
				<br />
				<br />
			</div>
		</Show>
	);
}
