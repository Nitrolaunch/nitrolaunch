import { useNavigate, useParams } from "@solidjs/router";
import { createEffect, createResource, createSignal, For, Match, onMount, Show, Switch } from "solid-js";
import { loadPagePlugins } from "../../plugins";
import { errorToast, successToast } from "../../components/dialog/Toasts";
import { beautifyString, getAccountIcon } from "../../utils";
import { Check, Delete, Info, Login, Logout, Star, User } from "../../icons";
import "./AccountPage.css";
import IconTextButton from "../../components/input/button/IconTextButton";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { AccountInfo } from "../../components/account/AccountWidget";
import { emit, Event, listen } from "@tauri-apps/api/event";
import InlineSelect from "../../components/input/select/InlineSelect";
import Icon from "../../components/Icon";
import { SkinViewer } from "skinview3d";
import Tip from "../../components/dialog/Tip";
import Dropdown from "../../components/input/select/Dropdown";
import SearchBar from "../../components/input/text/SearchBar";
import { undefinedEmpty } from "../../utils/values";

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
			setPreviewedSkin(undefined);
			setPreviewedCape(undefined);
			return accounts[id()];
		} catch (e) {
			errorToast("Failed to get account: " + e);
			return undefined;
		}
	});

	let [cosmetics, cosmeticsMethods] = createResource(() => account(), async (account) => {
		// Don't auth now if we aren't logged in
		if (account.username == undefined) {
			return [[], []];
		}
		try {
			let cosmetics = await invoke("get_cosmetics", { account: id() }) as [Skin[], Cape[]];

			// Set previews
			for (let skin of cosmetics[0]) {
				if (skin.state == "ACTIVE") {
					setPreviewedSkin(skin);
				}
			}
			for (let cape of cosmetics[1]) {
				if (cape.state == "ACTIVE") {
					setPreviewedCape(cape.url);
				}
			}

			setSkins(cosmetics[0]);
			return cosmetics;
		} catch (e) {
			errorToast("Failed to fetch cosmetics: " + e);
			return [[], []];
		}
	}, { initialValue: [[], []] });

	let [cosmeticType, setCosmeticType] = createSignal("skin");

	let [skinViewer, setSkinViewer] = createSignal<SkinViewer | undefined>(undefined);

	let skinViewerElem!: HTMLCanvasElement;
	onMount(() => {
		if (skinViewerElem == undefined) {
			return;
		}
		setSkinViewer(new SkinViewer({
			canvas: skinViewerElem,
			width: skinViewerElem.getBoundingClientRect().width,
			height: skinViewerElem.getBoundingClientRect().height,
		}));
	});

	let [skins, setSkins] = createSignal<Skin[]>([]);

	let [previewedSkin, setPreviewedSkin] = createSignal<Skin | undefined>();
	let [previewedCape, setPreviewedCape] = createSignal<string | undefined>();

	let [search, setSearch] = createSignal<string | undefined>();
	let [selectedRepo, setSelectedRepo] = createSignal<string | undefined>();

	let [availableRepos, _] = createResource(async () => {
		try {
			return await invoke("get_skin_repositories") as SkinRepository[];
		} catch (e) {
			errorToast("Failed to get skin repositories: " + e);
			return [];
		}
	}, { initialValue: [] });

	createEffect(async () => {
		if (selectedRepo() != undefined) {
			try {
				let skins = await invoke("search_skins", { repository: selectedRepo()!, search: search() }) as Skin[];
				setSkins(skins);
			} catch (e) {
				errorToast("Failed to search skins: " + e);
			}
		} else {
			setSkins(cosmetics()[0]);
		}
	})

	createEffect(() => {
		if (skinViewer() != undefined) {
			if (previewedSkin() == undefined) {
				skinViewer()!.resetSkin();
			} else {
				skinViewer()!.loadSkin(previewedSkin()!.url, { model: previewedSkin()!.variant == "CLASSIC" ? "default" : "slim" });
			}

			if (previewedCape() == undefined) {
				skinViewer()!.resetCape();
			} else {
				skinViewer()!.loadCape(previewedCape()!);
			}

			skinViewer()!.render();
		}
	})

	return (
		<div class="cont col" style="width:100%">
			<div class="cont col" id="account-container">
				<div class="cont" id="account-header-container">
					<div class="shadow" id="account-header">
						<div class="cont start" id="account-icon">
							<img
								id="account-icon-image"
								src={account() == undefined ? getAccountIcon(undefined) : getAccountIcon(account()!.uuid)}
								onerror={(e) =>
									((e.target as any).src = getAccountIcon(undefined))
								}
							/>
						</div>
						<div id="account-details-container">
							<div class="col" id="account-details">
								<div class="cont" id="account-upper-details">
									<div id="account-name">
										{account() == undefined || account()!.username == undefined
											? id()
											: account()!.username}
									</div>
									<Show when={account() != undefined && account()!.username != undefined}>
										<div id="account-id">{id()}</div>
									</Show>
								</div>
								<div class="cont start" id="account-lower-details">
									<div class="cont" id="account-type">
										{beautifyString(account() == undefined ? "Unknown" : account()!.type).toLocaleUpperCase()}
									</div>
								</div>
							</div>
							<div class="cont end" style="margin-right:1rem">
								<Switch>
									<Match when={account() != undefined && account()!.username == undefined}>
										<IconTextButton
											icon={Login}
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
									<Match when={account() != undefined && account()!.username != undefined}>
										<IconTextButton
											icon={Logout}
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
					<div class="split fullwidth">
						<div class="cont start">
							<div class="cont" style="width:20rem">
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
							<div class="cont start bold" style="color:var(--fg3);text-wrap:nowrap">
								<Icon icon={Info} size="1rem" />
								Click any cosmetic to preview
							</div>
						</div>
						<div class="cont end">
							<SearchBar value={search()} method={(x) => setSearch(undefinedEmpty(x))} />
							<div style="width:10rem">
								<Dropdown
									options={availableRepos().map((x) => {
										return {
											value: x.id,
											contents: x.name,
										}
									})}
									selected={selectedRepo()}
									onChange={setSelectedRepo}
									allowEmpty
								/>
							</div>
						</div>
					</div>
					<div id="cosmetics-container">
						<Switch>
							<Match when={account() != undefined && account()!.username == undefined}>
								<span class="cont fullwidth" style="color:var(--fg2)">
									Log in to see skins and capes
								</span>
							</Match>
							<Match when={account() == undefined || account()!.username != undefined}>
								<div id="cosmetics">
									<Show when={cosmeticType() == "skin"}>
										<For each={skins()}>
											{
												(skin) => <Cosmetic
													id={skin.id}
													url={skin.url}
													state={skin.state}
													skinVariant={skin.variant}
													capeAlias={undefined}
													isPreviewed={previewedSkin() != undefined && previewedSkin()!.id == skin.id}
													onClick={() => setPreviewedSkin(skin)}
													accountId={id()}
													refetchCosmetics={cosmeticsMethods.refetch}
												/>
											}
										</For>
									</Show>
									<Show when={cosmeticType() == "cape"}>
										<For each={cosmetics()[1]}>
											{
												(cape) => <Cosmetic
													id={cape.id}
													url={cape.url}
													state={cape.state}
													skinVariant={undefined}
													capeAlias={cape.alias}
													isPreviewed={previewedCape() == cape.url}
													onClick={() => setPreviewedCape(cape.url)}
													accountId={id()}
													refetchCosmetics={cosmeticsMethods.refetch}
												/>
											}
										</For>
									</Show>
								</div>
							</Match>
						</Switch>
						<div class="cont col" id="cosmetic-preview-container">
							<canvas id="cosmetic-preview" ref={skinViewerElem}></canvas>
						</div>
					</div>
				</div>
			</div>
			<br />
			<br />
			<br />
		</div>
	);
}

function Cosmetic(props: CosmeticProps) {
	let displayName = props.capeAlias == undefined ? props.id.split("-")[0] : props.capeAlias;
	let isActive = () => props.state == "ACTIVE";
	let isSkin = props.capeAlias == undefined;
	let color = isSkin ? "var(--instance)" : "var(--warning)";

	let img = props.url == undefined ? convertFileSrc(props.path!) : props.url!;

	let [isHovered, setIsHovered] = createSignal(false);

	async function activate() {
		props.onClick();

		try {
			if (isSkin) {
				let uri = props.url == undefined ? props.path! : props.url!;
				await invoke("upload_skin", { account: props.accountId, skinUri: uri, variant: props.skinVariant });
				successToast("Skin uploaded");
			} else {
				let cape = isActive() ? undefined : props.id;
				await invoke("activate_cape", { account: props.accountId, cape: cape });
				successToast("Cape updated");
			}
			props.refetchCosmetics();
		} catch (e) {
			errorToast("Failed to activate cosmetic: " + e);
		}
	}

	return <div
		class={`cont col cosmetic ${isSkin ? "skin" : "cape"} ${props.isPreviewed ? "preview" : ""} `}
		onclick={props.onClick}
		onmouseenter={() => setIsHovered(true)}
		onmouseleave={() => setIsHovered(false)}
	>
		<Show when={isActive()}>
			<div class="cont shadow cosmetic-active">
				<Tip side="top" tip="Currently Active" fullwidth cont>
					<span class="cont" style="color:var(--bg)">
						<Icon icon={Check} size="1.2rem" />
					</span>
				</Tip>
			</div>
		</Show>
		<img class="cosmetic-thumbnail" src={img} />
		<div class="split fullwidth cosmetic-details">
			<div class="cont start">
				{displayName}
			</div>
			<div class="cont end">
				<Show when={isHovered()}>
					<IconTextButton
						size="1rem"
						text={!isSkin && isActive() ? "Deactivate" : "Activate"}
						onClick={activate}
						bgColor={color}
						color="black"
					/>
				</Show>
			</div>
		</div>
	</div>
}

interface CosmeticProps {
	id: string;
	url?: string;
	path?: string;
	state: "ACTIVE" | "INACTIVE";
	skinVariant: "CLASSIC" | "SLIM" | undefined;
	capeAlias: string | undefined;
	isPreviewed: boolean;
	onClick: () => void;
	accountId: string;
	refetchCosmetics: () => void;
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

interface SkinRepository {
	id: string;
	name: string;
}
