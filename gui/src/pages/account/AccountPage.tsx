import { useNavigate, useParams } from "@solidjs/router";
import { createResource, createSignal, For, Match, onMount, Show, Switch } from "solid-js";
import { loadPagePlugins } from "../../plugins";
import { errorToast, successToast } from "../../components/dialog/Toasts";
import LoadingSpinner from "../../components/utility/LoadingSpinner";
import { beautifyString, getAccountIcon } from "../../utils";
import { Delete, Lock, LockOpen, Star, User } from "../../icons";
import "./AccountPage.css";
import IconTextButton from "../../components/input/button/IconTextButton";
import { invoke } from "@tauri-apps/api/core";
import { AccountInfo } from "../../components/account/AccountWidget";
import { emit, Event, listen } from "@tauri-apps/api/event";
import InlineSelect from "../../components/input/select/InlineSelect";
import Icon from "../../components/Icon";

export default function AccountPage() {
	let navigate = useNavigate();

	let params = useParams();
	let id = () => params.accountId;

	let [account, accountOperations] = createResource(id, async () => {
		try {
			let [_, accounts] = (await invoke("get_accounts")) as [
				string | undefined,
				{ [id: string]: AccountInfo },
			];

			loadPagePlugins("account", id());
			return accounts[id()];
		} catch (e) {
			errorToast("Failed to get account: " + e);
			return undefined;
		}
	});

	let [cosmetics, cosmeticOperations] = createResource(() => account(), async (account) => {
		// Don't auth now if we aren't logged in
		if (account.username == undefined) {
			return [[], []];
		}
		try {
			return await invoke("get_cosmetics", { account: id() }) as [Skin[], Cape[]];
		} catch (e) {
			errorToast("Failed to fetch cosmetics: " + e);
			return [[], []];
		}
	}, { initialValue: [[], []] });

	let [cosmeticType, setCosmeticType] = createSignal("skin");

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
												? id()
												: account()!.username}
										</div>
										<Show when={account()!.username != undefined}>
											<div id="account-id">{id()}</div>
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
														await invoke("login_account", { account: id() });

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
														await invoke("logout_account", { account: id() });
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
													account: id(),
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
					<div id="account-body" class="shadow">
						<div class="cont start" style="width:20rem">
							<InlineSelect
								options={[
									{
										value: "skin",
										contents: <div class="cont"><Icon icon={User} size="1rem" />Skins</div>,
										color: "var(--instance)",
									},
									{
										value: "cape",
										contents: <div class="cont"><Icon icon={Star} size="1rem" />Capes</div>,
										color: "var(--warning)",
									},
								]}
								selected={cosmeticType()}
								columns={2}
								onChange={setCosmeticType}
								solidSelect
							/>
						</div>
						<div id="cosmetics">
							<Show when={cosmeticType() == "skin"}>
								<For each={cosmetics()[0]}>
									{(skin) => <Cosmetic id={skin.id} url={skin.url} state={skin.state} skinVariant={skin.variant} capeAlias={undefined} />}
								</For>
							</Show>
							<Show when={cosmeticType() == "cape"}>
								<For each={cosmetics()[1]}>
									{(cape) => <Cosmetic id={cape.id} url={cape.url} state={cape.state} skinVariant={undefined} capeAlias={cape.alias} />}
								</For>
							</Show>
						</div>
						<Show when={account() != undefined && account()!.username == undefined}>
							<span class="cont fullwidth" style="color:var(--fg2)">
								Log in to see skins and capes
							</span>
						</Show>
					</div>
				</div>
				<br />
				<br />
				<br />
			</div>
		</Show>
	);
}

function Cosmetic(props: CosmeticProps) {
	let displayName = props.capeAlias == undefined ? props.id.split("-")[0] : props.capeAlias;

	return <div class={`cont col cosmetic ${props.skinVariant == undefined ? "cape" : "skin"} ${props.state == "ACTIVE" ? "active" : ""} `}>
		<img class="cosmetic-thumbnail" src={props.url} />
		{displayName}
	</div>
}

interface CosmeticProps {
	id: string;
	url: string;
	state: "ACTIVE" | "INACTIVE";
	skinVariant: "CLASSIC" | "SLIM" | undefined;
	capeAlias: string | undefined;
}

interface Skin {
	id: string;
	url: string;
	state: "ACTIVE" | "INACTIVE";
	variant: "CLASSIC" | "SLIM";
}

interface Cape {
	id: string;
	url: string;
	state: "ACTIVE" | "INACTIVE";
	alias: string;
}
