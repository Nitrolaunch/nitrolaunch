import { invoke } from "@tauri-apps/api/core";
import {
	createResource,
	createSignal,
	For,
	Match,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import {
	AngleDown,
	AngleRight,
	Check,
	Plus,
	Properties,
	User,
} from "../../icons";
import "./AccountWidget.css";
import { stringCompare, getAccountIcon } from "../../utils";
import Icon from "../Icon";
import { errorToast, successToast } from "../dialog/Toasts";
import IconButton from "../input/button/IconButton";
import { listen } from "@tauri-apps/api/event";
import { sanitizeInstanceId } from "../../pages/instance/InstanceConfig";
import Dropdown from "../input/select/Dropdown";
import { clearInputError, inputError } from "../../errors";
import { useNavigate } from "@solidjs/router";
import Modal from "../dialog/Modal";
import { AccountTypeInfo } from "../../types";

export default function AccountWidget() {
	let navigate = useNavigate();

	let [accountData, methods] = createResource(async () => {
		try {
			let [currentAccount, accounts] = (await invoke("get_accounts")) as [
				string | undefined,
				AccountMap,
			];

			let currentAccountInfo =
				currentAccount == undefined ? undefined : accounts[currentAccount];

			let accountList = [];
			for (let account of Object.values(accounts)) {
				if (account != undefined) {
					accountList.push(account);
				}
			}
			accountList.sort((a, b) => stringCompare(a.id, b.id));

			return {
				currentAccount: currentAccountInfo,
				accounts: accountList,
			} as AccountData;
		} catch (e) {
			errorToast("Failed to get accounts: " + e);
			return undefined;
		}
	});

	let [isOpen, setIsOpen] = createSignal(false);

	let [isCreatingAccount, setIsCreatingAccount] = createSignal(false);
	let [newAccountId, setNewAccountId] = createSignal("");
	let [newAccountType, setNewAccountType] = createSignal("microsoft");

	let [eventUnlisten, _] = createResource(async () => {
		return await listen("refresh_accounts", () => {
			methods.refetch();
		});
	});

	onCleanup(() => {
		if (eventUnlisten() != undefined) {
			eventUnlisten()!();
		}
	});

	let [availableAccountTypes, __] = createResource(
		async () => {
			let pluginTypes = (await invoke(
				"get_supported_account_types",
			)) as AccountTypeInfo[];

			let out: AccountTypeInfo[] = [
				{
					id: "microsoft",
					name: "Microsoft",
					color: "#00a2ed",
				},
				{
					id: "demo",
					name: "Demo",
					color: "#dddddd",
				},
			];
			out = out.concat(pluginTypes);

			return out;
		},
		{ initialValue: [] },
	);

	return (
		<div id="account-widget" onmouseleave={() => setIsOpen(false)}>
			<div
				id="account-widget-head"
				class={`${isOpen() ? "open" : "bubble-hover"}`}
				onclick={() => setIsOpen(!isOpen())}
			>
				<Show
					when={
						accountData() != undefined &&
						accountData()!.currentAccount != undefined
					}
					fallback={
						<AccountTile
							account={{
								id: "",
								type: "other",
								username: "No Account Selected",
							}}
							isFeatured={true}
							onclick={() => { }}
							onClose={() => setIsOpen(false)}
						/>
					}
				>
					<AccountTile
						account={accountData()!.currentAccount!}
						isFeatured={true}
						onclick={() => { }}
						onClose={() => setIsOpen(false)}
					/>
				</Show>
				<div class="cont" id="account-widget-dropdown-button">
					<Switch>
						<Match when={!isOpen()}>
							<AngleRight />
						</Match>
						<Match when={isOpen()}>
							<AngleDown />
						</Match>
					</Switch>
				</div>
			</div>
			<Show when={isOpen()}>
				<div class="shadow" id="account-widget-dropdown">
					<Show when={accountData() != undefined}>
						<For each={accountData()!.accounts}>
							{(account) => (
								<Show when={account != undefined}>
									<div class="account-widget-dropdown-item">
										<AccountTile
											account={account!}
											isFeatured={false}
											onclick={(account) => {
												invoke("select_account", { account: account }).then(
													() => {
														methods.refetch();
														setIsOpen(false);
													},
												);
											}}
											onClose={() => setIsOpen(false)}
										/>
									</div>
								</Show>
							)}
						</For>
					</Show>
					<div
						class="bubble-hover account-tile"
						onclick={() => {
							setIsCreatingAccount(true);
							setIsOpen(false);
						}}
					>
						<div class="cont">
							<Icon icon={Plus} size="1.2rem" />
						</div>
						<div class="cont account-tile-name">Add Account</div>
					</div>
				</div>
			</Show>
			<Modal
				visible={isCreatingAccount()}
				onClose={setIsCreatingAccount}
				title="Create new account"
				titleIcon={User}
				buttons={[
					{
						text: "Save",
						icon: Check,
						onClick: async () => {
							if (newAccountId() == "") {
								inputError("new-account-id", "ID cannot be empty");
								return;
							} else {
								clearInputError("new-account-id");
							}

							try {
								await invoke("create_account", {
									id: newAccountId(),
									kind: newAccountType(),
								});
								setIsCreatingAccount(false);
								navigate(`/accounts/${newAccountId()}`);
								successToast("Account created");
							} catch (e) {
								setIsCreatingAccount(false);
								errorToast("Failed to create account: " + e);
							}
						},
					},
				]}
			>
				<div class="cont col fullwidth">
					<label class="cont start fullwidth label" for="account-id">
						ID
					</label>
					<input
						type="text"
						name="account-id"
						id="new-account-id"
						placeholder="Enter account ID..."
						style="width:100%"
						onChange={(e) => {
							e.target.value = sanitizeInstanceId(e.target.value);
							setNewAccountId(e.target.value);
						}}
						onkeyup={(e: any) => {
							e.target.value = sanitizeInstanceId(e.target.value);
						}}
					/>
				</div>
				<label class="cont start fullwidth label" for="account-id">
					TYPE
				</label>
				<Dropdown
					options={availableAccountTypes().map((x) => {
						return {
							value: x.id,
							contents: x.name,
							color: x.color,
						};
					})}
					selected={newAccountType()}
					onChange={setNewAccountType}
					zIndex="200"
					isSearchable={false}
				/>
				<div></div>
			</Modal>
		</div>
	);
}

interface AccountData {
	currentAccount?: AccountInfo;
	accounts: AccountInfo[];
}

type AccountMap = { [id: string]: AccountInfo | undefined };

export interface AccountInfo {
	id: string;
	type: "microsoft" | "demo" | "other";
	username?: string;
	uuid?: string;
}

function AccountTile(props: AccountTileProps) {
	let navigate = useNavigate();

	let [isHovered, setIsHovered] = createSignal(false);

	return (
		<div
			class={`account-tile ${isHovered() && !props.isFeatured ? "hover" : ""}`}
			onclick={() => {
				if (!props.isFeatured) {
					props.onclick(props.account.id);
				}
			}}
			onmouseenter={() => setIsHovered(true)}
			onmouseleave={() => setIsHovered(false)}
		>
			<div class="cont">
				<img
					class="account-tile-image"
					src={getAccountIcon(props.account.uuid)}
					onerror={(e) => (e.target as any).src = "/default_skin.png"}
				/>
			</div>
			<div class="cont account-tile-name">
				{props.account.username == undefined
					? props.account.id
					: props.account.username}
			</div>
			<Show when={!props.isFeatured && isHovered()}>
				<div class="cont account-tile-edit">
					<IconButton
						icon={Properties}
						size="1.25rem"
						color="var(--bg)"
						border="var(--bg4)"
						selectedColor=""
						hoverBorder="var(--fg3)"
						selected={false}
						onClick={(e) => {
							e.preventDefault();
							e.stopPropagation();

							props.onClose();

							navigate(`/accounts/${props.account.id}`);
						}}
					/>
				</div>
			</Show>
		</div>
	);
}

interface AccountTileProps {
	account: AccountInfo;
	isFeatured: boolean;
	onclick: (account: string) => void;
	onClose: () => void;
}
