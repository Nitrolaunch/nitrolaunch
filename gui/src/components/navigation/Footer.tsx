import {
	For,
	Show,
	createResource,
	createSignal,
	onCleanup,
	onMount,
} from "solid-js";
import "./Footer.css";
import { UnlistenFn, listen, Event, emit } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api";
import { PasswordPrompt } from "../input/PasswordPrompt";
import {
	Box,
	Check,
	Download,
	Gear,
	Play,
	Properties,
	Trash,
	Upload,
} from "../../icons";
import IconButton from "../input/IconButton";
import { AuthDisplayEvent } from "../../types";
import MicrosoftAuthInfo from "../input/MicrosoftAuthInfo";
import { beautifyString } from "../../utils";
import TaskIndicator from "../TaskIndicator";
import { errorToast } from "../dialog/Toasts";
import Tip from "../dialog/Tip";
import ProfileDeletePrompt from "../instance/ProfileDeletePrompt";
import RunningInstanceList from "../launch/RunningInstanceList";

export default function Footer(props: FooterProps) {
	// Prompts
	const [showPasswordPrompt, setShowPasswordPrompt] = createSignal(false);
	const [authInfo, setAuthInfo] = createSignal<AuthDisplayEvent | undefined>(
		undefined
	);
	const [passwordPromptMessage, setPasswordPromptMessage] = createSignal("");
	let [showProfileDeletePrompt, setShowProfileDeletePrompt] =
		createSignal(false);

	// Unlisteners for tauri events
	const [unlistens, setUnlistens] = createSignal<UnlistenFn[]>([]);

	// Setup and clean up event listeners for updating state
	onMount(async () => {
		for (let unlisten of unlistens()) {
			unlisten();
		}

		let authInfoPromise = listen(
			"nitro_display_auth_info",
			(event: Event<AuthDisplayEvent>) => {
				setAuthInfo(event.payload);
			}
		);

		let authInfoClosePromise = listen("nitro_close_auth_info", () => {
			setAuthInfo(undefined);
		});

		let passwordPromise = listen(
			"nitro_display_password_prompt",
			(event: Event<string>) => {
				setShowPasswordPrompt(true);
				setPasswordPromptMessage(event.payload);
			}
		);

		let eventUnlistens = await Promise.all([
			authInfoPromise,
			authInfoClosePromise,
			passwordPromise,
		]);

		setUnlistens(eventUnlistens);

		(window as any).__launchInstance = launch;
	});

	onCleanup(() => {
		for (const unlisten of unlistens()) {
			unlisten();
		}
	});

	async function launch(instance: string, offline: boolean) {
		// Prevent launching until the current authentication screens are finished
		if (showPasswordPrompt() || authInfo() !== undefined) {
			return;
		}

		emit("refresh_users");

		let launchPromise = invoke("launch_game", {
			instanceId: instance,
			offline: offline,
			user: props.selectedUser,
		});

		try {
			await Promise.all([launchPromise]);
		} catch (e) {
			errorToast("Failed to launch instance: " + e);
		}
	}

	// Gets whether the currently selected instance is launchable (it has been updated before)
	let [isInstanceLaunchable, methods] = createResource(
		() => props.selectedItem,
		async () => {
			if (props.mode != FooterMode.Instance) {
				return undefined;
			}

			let unlisten = await listen(
				"nitro_output_finish_task",
				(e: Event<string>) => {
					if (e.payload == "update_instance") {
						methods.refetch();
					}
				}
			);

			setUnlistens((unlistens) => {
				unlistens.push(unlisten);
				return unlistens;
			});

			return await invoke("get_instance_has_updated", {
				instance: props.selectedItem,
			});
		}
	);

	let gallery!: HTMLDivElement;

	let scrollFunction = (e: any) => {
		if (!e.deltaY) {
			return;
		}

		gallery.scrollLeft += e.deltaY;
		e.preventDefault();
		e.stopPropagation();
	};

	let [hoveredGalleryEntry, setHoveredGalleryEntry] = createSignal<
		string | undefined
	>();

	return (
		<div class="footer">
			<div id="footer-left" class="footer-section">
				<div class="cont" id="footer-selection-indicator">
					<Show
						when={props.selectedItem != undefined && props.selectedItem != ""}
					>
						{`Selected: ${props.selectedItem}`}
					</Show>
				</div>
				<div class="cont" style="position:relative">
					<Show when={props.selectedPackageGallery != undefined}>
						<div
							class="cont start"
							id="footer-package-gallery"
							ref={gallery}
							onwheel={scrollFunction}
						>
							<For each={props.selectedPackageGallery}>
								{(url) => (
									<img
										class="footer-package-gallery-entry"
										src={url}
										onwheel={scrollFunction}
										onmouseenter={() => setHoveredGalleryEntry(url)}
										onmouseleave={() => setHoveredGalleryEntry(undefined)}
									/>
								)}
							</For>
						</div>
						<Show when={hoveredGalleryEntry() != undefined}>
							<div
								class="cont fade-in-fast"
								id="footer-package-gallery-preview"
							>
								<img src={hoveredGalleryEntry()} />
							</div>
						</Show>
					</Show>
				</div>
			</div>
			<div id="footer-center" class="cont footer-section">
				<div id="footer-center-inner">
					<div class="cont" id="footer-left-buttons">
						<Show
							when={
								props.mode == FooterMode.Instance &&
								props.selectedItem != undefined
							}
							fallback={<div></div>}
						>
							<div class="cont footer-update">
								<Tip tip="Update instance" side="top">
									<IconButton
										icon={Upload}
										size="1.5rem"
										color="var(--bg0)"
										selectedColor="var(--accent)"
										onClick={async () => {
											if (props.selectedItem != undefined) {
												try {
													await invoke("update_instance", {
														instanceId: props.selectedItem,
														depth: "full",
													});
												} catch (e) {
													errorToast("Failed to update instance: " + e);
												}
											}
										}}
										selected={false}
										circle
										hoverBackground="var(--bg3)"
									/>
								</Tip>
							</div>
							<Show when={props.itemFromPlugin != true}>
								<div class="cont footer-config">
									<Tip tip="View instance" side="top">
										<IconButton
											icon={Properties}
											size="1.5rem"
											color="var(--bg0)"
											selectedColor="var(--accent)"
											onClick={() => {
												window.location.href = `/instance/${props.selectedItem}`;
											}}
											selected={false}
											circle
											hoverBackground="var(--bg3)"
										/>
									</Tip>
								</div>
							</Show>
						</Show>
						<Show
							when={
								props.mode == FooterMode.Profile &&
								props.selectedItem != undefined
							}
						>
							<div class="cont">
								<Tip tip="Delete profile" side="top">
									<IconButton
										icon={Trash}
										size="1.5rem"
										color="var(--bg0)"
										selectedColor="var(--accent)"
										onClick={() => {
											setShowProfileDeletePrompt(true);
										}}
										selected={false}
										circle
										hoverBackground="var(--bg3)"
									/>
								</Tip>
							</div>
						</Show>
					</div>
					<ActionButton
						selected={
							props.selectedItem != undefined &&
							!(
								props.itemFromPlugin == true && props.mode == FooterMode.Profile
							) &&
							!(
								props.mode == FooterMode.Instance &&
								isInstanceLaunchable() == undefined
							)
						}
						mode={props.mode}
						isInstanceLaunchable={isInstanceLaunchable() == true}
						onClick={async () => {
							if (props.mode == FooterMode.Instance) {
								if (props.selectedItem != undefined) {
									if (isInstanceLaunchable() == undefined) {
									} else if (isInstanceLaunchable()) {
										launch(props.selectedItem, false);
									} else {
										try {
											await invoke("update_instance", {
												instanceId: props.selectedItem,
												depth: "full",
											});
										} catch (e) {
											errorToast("Failed to update instance: " + e);
										}
									}
								}
							} else if (
								props.mode == FooterMode.Profile &&
								props.itemFromPlugin != true
							) {
								if (props.selectedItem != undefined) {
									window.location.href = `/profile_config/${props.selectedItem}`;
								}
							} else {
								props.action();
							}
						}}
					/>
				</div>
				<div class="cont">
					<Show when={props.itemFromPlugin == true}>
						<Tip
							tip={`This ${beautifyString(
								props.mode
							)} is from a plugin and cannot be edited`}
							side="top"
						>
							<div class="cont footer-plugin-indicator">P</div>
						</Tip>
					</Show>
				</div>
			</div>
			<div id="footer-right" class="footer-section fullheight">
				<div class="cont">
					<TaskIndicator />
				</div>
				<div class="cont fullheight">
					<RunningInstanceList />
				</div>
			</div>

			<MicrosoftAuthInfo
				visible={authInfo() != undefined}
				event={authInfo() as AuthDisplayEvent}
				onCancel={() => {
					setAuthInfo(undefined);
				}}
			/>
			<Show when={showPasswordPrompt()}>
				<PasswordPrompt
					onSubmit={() => setShowPasswordPrompt(false)}
					message={passwordPromptMessage()}
				/>
			</Show>
			<ProfileDeletePrompt
				visible={showProfileDeletePrompt()}
				onClose={() => setShowProfileDeletePrompt(false)}
				profile={
					props.mode != FooterMode.Profile ? undefined : props.selectedItem
				}
			/>
		</div>
	);
}

export interface FooterProps {
	selectedItem?: string;
	mode: FooterMode;
	selectedUser?: string;
	action: () => void;
	itemFromPlugin?: boolean;
	selectedPackageGallery?: string[];
}

function ActionButton(props: ActionButtonProps) {
	let backgroundColor = () => {
		if (props.selected) {
			if (
				props.mode == FooterMode.Instance ||
				props.mode == FooterMode.SaveInstanceConfig
			) {
				return "var(--instancebg)";
			} else if (
				props.mode == FooterMode.Profile ||
				props.mode == FooterMode.SaveProfileConfig ||
				props.mode == FooterMode.SaveSettings
			) {
				return "var(--profilebg)";
			} else if (props.mode == FooterMode.PreviewPackage) {
				return "var(--bg-1)";
			} else if (props.mode == FooterMode.InstallPackage) {
				return "var(--packagebg)";
			}
		}
		return "var(--bg-1)";
	};
	let borderColor = () => {
		if (props.selected) {
			if (
				props.mode == FooterMode.Instance ||
				props.mode == FooterMode.SaveInstanceConfig
			) {
				return "var(--instance)";
			} else if (
				props.mode == FooterMode.Profile ||
				props.mode == FooterMode.SaveProfileConfig ||
				props.mode == FooterMode.SaveSettings
			) {
				return "var(--profile)";
			} else if (
				props.mode == FooterMode.PreviewPackage ||
				props.mode == FooterMode.InstallPackage
			) {
				return "var(--package)";
			}
		}
		return "var(--fg3)";
	};
	let message = () => {
		if (props.mode == FooterMode.Instance) {
			if (props.isInstanceLaunchable || !props.selected) {
				return "Launch";
			} else {
				return "Update";
			}
		} else if (props.mode == FooterMode.Profile) {
			return "Edit";
		} else if (
			props.mode == FooterMode.SaveInstanceConfig ||
			props.mode == FooterMode.SaveProfileConfig ||
			props.mode == FooterMode.SaveSettings
		) {
			return "Save";
		} else if (props.mode == FooterMode.PreviewPackage) {
			return "Open";
		} else if (props.mode == FooterMode.InstallPackage) {
			return "Install";
		}
	};
	let Icon = () => {
		if (props.mode == FooterMode.Instance) {
			if (props.isInstanceLaunchable || !props.selected) {
				return <Play />;
			} else {
				return <Upload />;
			}
		} else if (props.mode == FooterMode.Profile) {
			return <Gear />;
		} else if (
			props.mode == FooterMode.SaveInstanceConfig ||
			props.mode == FooterMode.SaveProfileConfig ||
			props.mode == FooterMode.SaveSettings
		) {
			return <Check />;
		} else if (props.mode == FooterMode.PreviewPackage) {
			return <Box />;
		} else if (props.mode == FooterMode.InstallPackage) {
			return <Download />;
		}
	};

	let backgroundStyle = () => `background-color:${backgroundColor()}`;
	let borderStyle = () => `border-color:${borderColor()}`;

	return (
		<div id="footer-action-button-container">
			<div class="footer-action-button-decorations">
				<div
					class="footer-action-button-decoration left"
					style={`${backgroundStyle()};${borderStyle()};${
						props.selected ? "" : "border-top-width:0px"
					}`}
				></div>
				<div
					class="footer-action-button-decoration right"
					style={borderStyle()}
				></div>
				<div
					class="footer-action-button-decoration left"
					style={`${backgroundStyle()};${borderStyle()}`}
				></div>
				<div
					class="footer-action-button-decoration right"
					style={borderStyle()}
				></div>
			</div>
			<div
				id="footer-action-button"
				class="cont"
				onclick={props.onClick}
				style={`background-color:${backgroundColor()};border-color:${borderColor()};color:${borderColor()};${
					props.selected ? "border-top: var(--border) solid" : ""
				}`}
			>
				{Icon()}
				{message()}
			</div>
			<div class="footer-action-button-decorations">
				<div
					class="footer-action-button-decoration left"
					style={`border-top-width:0px;border-top-left-radius:0px;${borderStyle()}`}
				></div>
				<div
					class="footer-action-button-decoration right"
					style={`${backgroundStyle()};${borderStyle()}`}
				></div>
				<div
					class="footer-action-button-decoration left"
					style={borderStyle()}
				></div>
				<div
					class="footer-action-button-decoration right"
					style={`${backgroundStyle()};${borderStyle()}`}
				></div>
			</div>
		</div>
	);
}

interface ActionButtonProps {
	selected: boolean;
	mode: FooterMode;
	isInstanceLaunchable: boolean;
	onClick: () => void;
}

// The mode for the footer
export enum FooterMode {
	Instance = "instance",
	Profile = "profile",
	SaveInstanceConfig = "save_instance_config",
	SaveProfileConfig = "save_profile_config",
	PreviewPackage = "preview_package",
	InstallPackage = "install_package",
	SaveSettings = "save_settings",
}

// Launches an instance
export function launchInstance(instance: string, offline: boolean) {
	(window as any).__launchInstance(instance, offline);
}
