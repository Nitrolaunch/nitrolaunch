import { invoke } from "@tauri-apps/api";
import {
	createResource,
	createSignal,
	For,
	Match,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import { AngleDown, AngleRight, Plus, Properties } from "../../icons";
import "./UserWidget.css";
import getUserIcon, { stringCompare } from "../../utils";
import Icon from "../Icon";
import { errorToast, successToast } from "../dialog/Toasts";
import IconButton from "../input/IconButton";
import { listen } from "@tauri-apps/api/event";
import Modal from "../dialog/Modal";
import { sanitizeInstanceId } from "../../pages/instance/InstanceConfig";
import Dropdown from "../input/Dropdown";
import IconTextButton from "../input/IconTextButton";
import { clearInputError, inputError } from "../../errors";

export default function UserWidget(props: UserWidgetProps) {
	let [userData, methods] = createResource(updateUsers);

	let [isOpen, setIsOpen] = createSignal(false);

	let [isCreatingUser, setIsCreatingUser] = createSignal(false);
	let [newUserId, setNewUserId] = createSignal("");
	let [newUserType, setNewUserType] = createSignal("microsoft");

	let [eventUnlisten, _] = createResource(async () => {
		return await listen("refresh_users", () => {
			methods.refetch();
		});
	});

	onCleanup(() => {
		if (eventUnlisten() != undefined) {
			eventUnlisten()!();
		}
	});

	async function updateUsers() {
		try {
			let [currentUser, users] = (await invoke("get_users")) as [
				string | undefined,
				UserMap
			];

			let currentUserInfo =
				currentUser == undefined ? undefined : users[currentUser];

			let userList = [];
			for (let user of Object.values(users)) {
				if (user != undefined) {
					userList.push(user);
				}
			}
			userList.sort((a, b) => stringCompare(a.id, b.id));

			return {
				currentUser: currentUserInfo,
				users: userList,
			} as UserData;
		} catch (e) {
			errorToast("Failed to get users: " + e);
			return undefined;
		}
	}

	return (
		<div id="user-widget" onmouseleave={() => setIsOpen(false)}>
			<div
				id="user-widget-head"
				class={isOpen() ? "open" : ""}
				onclick={() => setIsOpen(!isOpen())}
			>
				<Show
					when={userData() != undefined && userData()!.currentUser != undefined}
					fallback={
						<div class="cont" id="user-widget-placeholder">
							No User Selected
						</div>
					}
				>
					<UserTile
						user={userData()!.currentUser!}
						isFeatured={true}
						onclick={() => {}}
					/>
				</Show>
				<div class="cont" id="user-widget-dropdown-button">
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
				<div id="user-widget-dropdown">
					<Show when={userData() != undefined}>
						<For each={userData()!.users}>
							{(user) => (
								<Show when={user != undefined}>
									<div class="user-widget-dropdown-item">
										<UserTile
											user={user!}
											isFeatured={false}
											onclick={(user) => {
												invoke("select_user", { user: user }).then(() => {
													props.onSelect(user);
													methods.refetch();
													setIsOpen(false);
												});
											}}
										/>
									</div>
								</Show>
							)}
						</For>
					</Show>
					<div class="user-tile" onclick={() => setIsCreatingUser(true)}>
						<div class="cont">
							<Icon icon={Plus} size="1.2rem" />
						</div>
						<div class="cont user-tile-name">New User</div>
					</div>
				</div>
			</Show>
			<Modal
				visible={isCreatingUser()}
				width="25rem"
				onClose={setIsCreatingUser}
			>
				<div class="cont col" style="padding:2rem">
					<h3>Creating New User</h3>
					<div class="cont col" style="width:100%">
						<label class="cont start fullwidth label" for="user-id">
							ID
						</label>
						<input
							type="text"
							name="user-id"
							id="new-user-id"
							placeholder="Enter user ID..."
							style="width:100%"
							onChange={(e) => {
								e.target.value = sanitizeInstanceId(e.target.value);
								setNewUserId(e.target.value);
							}}
							onkeyup={(e: any) => {
								e.target.value = sanitizeInstanceId(e.target.value);
							}}
						/>
					</div>
					<label class="cont start fullwidth label" for="user-id">
						TYPE
					</label>
					<Dropdown
						options={[{ value: "microsoft", contents: "Microsoft" }]}
						selected={newUserType()}
						onChange={setNewUserType}
					/>
					<div></div>
					<IconTextButton
						text="Save"
						size="1rem"
						color="var(--bg2)"
						selectedColor="var(--bg3)"
						selected={false}
						onClick={async () => {
							if (newUserId() == "") {
								inputError("new-user-id", "ID cannot be empty");
							} else {
								clearInputError("new-user-id");
							}

							try {
								await invoke("create_user", {
									id: newUserId(),
									kind: newUserType(),
								});
								setIsCreatingUser(false);
								window.location.href = `/users/${newUserId()}`;
								successToast("User created");
							} catch (e) {
								setIsCreatingUser(false);
								errorToast("Failed to create user: " + e);
							}
						}}
					/>
				</div>
			</Modal>
		</div>
	);
}

export interface UserWidgetProps {
	onSelect: (user: string) => void;
}

interface UserData {
	currentUser?: UserInfo;
	users: UserInfo[];
}

type UserMap = { [id: string]: UserInfo | undefined };

export interface UserInfo {
	id: string;
	type: "microsoft" | "demo" | "other";
	username?: string;
	uuid?: string;
}

function UserTile(props: UserTileProps) {
	let [isHovered, setIsHovered] = createSignal(false);

	return (
		<div
			class={`user-tile ${isHovered() && !props.isFeatured ? "hover" : ""}`}
			onclick={() => {
				if (!props.isFeatured) {
					props.onclick(props.user.id);
				}
			}}
			onmouseenter={() => setIsHovered(true)}
			onmouseleave={() => setIsHovered(false)}
		>
			<div class="cont">
				<img class="user-tile-image" src={getUserIcon(props.user.uuid)} />
			</div>
			<div class="cont user-tile-name">
				{props.user.username == undefined ? props.user.id : props.user.username}
			</div>
			<Show when={!props.isFeatured && isHovered()}>
				<div class="cont user-tile-edit">
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

							window.location.href = `/users/${props.user.id}`;
						}}
					/>
				</div>
			</Show>
		</div>
	);
}

interface UserTileProps {
	user: UserInfo;
	isFeatured: boolean;
	onclick: (user: string) => void;
}
