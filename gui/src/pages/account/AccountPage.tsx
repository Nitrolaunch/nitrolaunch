import { useNavigate, useParams } from "@solidjs/router";
import { createResource, Match, onMount, Show, Switch } from "solid-js";
import { loadPagePlugins } from "../../plugins";
import { errorToast, successToast } from "../../components/dialog/Toasts";
import LoadingSpinner from "../../components/utility/LoadingSpinner";
import { beautifyString, getAccountIcon } from "../../utils";
import { Delete, Lock, LockOpen } from "../../icons";
import "./AccountPage.css";
import IconTextButton from "../../components/input/button/IconTextButton";
import { invoke } from "@tauri-apps/api/core";
import { AccountInfo } from "../../components/account/AccountWidget";
import { emit, Event, listen } from "@tauri-apps/api/event";

export default function AccountPage() {
	let navigate = useNavigate();

	let params = useParams();
	let id = params.accountId;

	onMount(() => loadPagePlugins("account", id));

	let [account, accountOperations] = createResource(async () => {
		try {
			let [_, accounts] = (await invoke("get_accounts")) as [
				string | undefined,
				{ [id: string]: AccountInfo },
			];

			return accounts[id];
		} catch (e) {
			errorToast("Failed to get account: " + e);
			return undefined;
		}
	});

	return (
		<Show
			when={account() != undefined}
			fallback={
				<div class="cont" style="width:100%">
					<LoadingSpinner size="5rem" />
				</div>
			}
		>
			<div class="cont col" style="width:100%">
				<div class="cont col" id="account-container">
					<div class="cont" id="account-header-container">
						<div class="shadow" id="account-header">
							<div class="cont start" id="account-icon">
								<img
									id="account-icon-image"
									src={getAccountIcon(account()!.uuid)}
									onerror={(e) =>
										((e.target as any).src = getAccountIcon(undefined))
									}
								/>
							</div>
							<div id="account-details-container">
								<div class="col" id="account-details">
									<div class="cont" id="account-upper-details">
										<div id="account-name">
											{account()!.username == undefined
												? id
												: account()!.username}
										</div>
										<Show when={account()!.username != undefined}>
											<div id="account-id">{id}</div>
										</Show>
									</div>
									<div class="cont start" id="account-lower-details">
										<div class="cont" id="account-type">
											{beautifyString(account()!.type).toLocaleUpperCase()}
										</div>
									</div>
								</div>
								<div class="cont end" style="margin-right:1rem">
									<Switch>
										<Match when={account()!.username == undefined}>
											<IconTextButton
												icon={LockOpen}
												size="1.2rem"
												text="Log In"
												onClick={async () => {
													try {
														await invoke("login_account", { account: id });

														let unlisten = await listen(
															"nitro_output_finish_task",
															(e: Event<string>) => {
																if (e.payload == "login_account") {
																	successToast("Logged in");
																	accountOperations.refetch();
																	emit("refresh_accounts");
																}
															},
														);

														unlisten();
													} catch (e) {
														errorToast("Failed to log in: " + e);
													}
												}}
												shadow={false}
											/>
										</Match>
										<Match when={account()!.username != undefined}>
											<IconTextButton
												icon={Lock}
												size="1.2rem"
												text="Log Out"
												onClick={async () => {
													try {
														await invoke("logout_account", { account: id });
														successToast("Logged out");
														accountOperations.refetch();
														emit("refresh_accounts");
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
										color="var(--error)"
										bgColor="var(--errorbg)"
										onClick={async () => {
											try {
												await invoke("remove_account", {
													account: id,
												});
												successToast("Account deleted");
												navigate("/");
											} catch (e) {
												errorToast("Failed to delete account: " + e);
											}
										}}
										shadow={false}
									/>
								</div>
							</div>
						</div>
					</div>
					<div id="account-body" class="shadow"></div>
				</div>
				<br />
				<br />
				<br />
			</div>
		</Show>
	);
}
