import { useNavigate, useParams } from "@solidjs/router";
import {
	createEffect,
	createResource,
	createSignal,
	Match,
	onCleanup,
	onMount,
	Show,
	Switch,
} from "solid-js";
import {
	dropdownButtonToOption,
	getDropdownButtons,
	loadPagePlugins,
	runDropdownButtonClick,
} from "../../plugins";
import {
	createConfiguredPackages,
	getConfigPackages,
	getParentTemplates,
	InstanceConfig,
	InstanceConfigMode,
	PackageOverrides,
	readEditableInstanceConfig,
	readInstanceConfig,
	saveInstanceConfig,
} from "./read_write";
import { errorToast, successToast } from "../../components/dialog/Toasts";
import LoadingSpinner from "../../components/utility/LoadingSpinner";
import { getInstanceIconSrc, parseVersionedString } from "../../utils";
import PackageLabels from "../../components/package/PackageLabels";
import { Loader } from "../../package";
import Icon from "../../components/Icon";
import {
	Box,
	Delete,
	Elipsis,
	Folder,
	Gear,
	Play,
	Popout,
	Stop,
	Tag,
	Text,
	Trash,
	Upload,
} from "../../icons";
import "./InstanceInfo.css";
import IconTextButton from "../../components/input/button/IconTextButton";
import { invoke } from "@tauri-apps/api/core";
import InstanceConsole from "../../components/launch/InstanceConsole";
import PackagesConfig, {
	PackageConfig,
	packageConfigsEqual,
	packageConfigsFullyEqual,
} from "./PackagesConfig";
import { FooterData } from "../../App";
import { FooterMode, launchInstance } from "../../components/navigation/Footer";
import { canonicalizeListOrSingle } from "../../utils/values";
import { Event, listen, UnlistenFn } from "@tauri-apps/api/event";
import { RunningInstancesEvent } from "../../components/launch/RunningInstanceList";
import { convertFileSrc } from "@tauri-apps/api/core";
import Dropdown, { Option } from "../../components/input/select/Dropdown";
import IconAndText from "../../components/utility/IconAndText";
import InstanceTransferPrompt from "../../components/instance/InstanceTransferPrompt";
import { updateInstanceList } from "./InstanceList";
import InstanceTiles from "../../components/instance/InstanceTiles";
import Modal from "../../components/dialog/Modal";

export default function InstanceInfo(props: InstanceInfoProps) {
	let navigate = useNavigate();

	let params = useParams();
	let id = params.instanceId;

	onMount(() => loadPagePlugins("instance", id));

	// Global, client, and server packages for the instance
	let [globalPackages, setGlobalPackages] = createSignal<PackageConfig[]>([]);
	let [clientPackages, setClientPackages] = createSignal<PackageConfig[]>([]);
	let [serverPackages, setServerPackages] = createSignal<PackageConfig[]>([]);
	// Derived packages
	let [derivedGlobalPackages, setDerivedGlobalPackages] = createSignal<
		PackageConfig[]
	>([]);
	let [derivedClientPackages, setDerivedClientPackages] = createSignal<
		PackageConfig[]
	>([]);
	let [derivedServerPackages, setDerivedServerPackages] = createSignal<
		PackageConfig[]
	>([]);

	createEffect(async () => {
		try {
			await invoke("set_last_opened_instance", {
				id: id,
				instanceOrTemplate: "instance",
			});
		} catch (e) {}
	});

	let [from, setFrom] = createSignal<string[] | undefined>();
	let [editableConfig, setEditableConfig] = createSignal<InstanceConfig>();
	let [instance, _] = createResource(async () => {
		// Get the instance or template
		try {
			let [configuration, editableConfiguration] = await Promise.all([
				readInstanceConfig(id, InstanceConfigMode.Instance),
				readEditableInstanceConfig(id, InstanceConfigMode.Instance),
			]);

			setFrom(canonicalizeListOrSingle(configuration.from));

			let [global, client, server] = getConfigPackages(editableConfiguration);
			setGlobalPackages(global);
			setClientPackages(client);
			setServerPackages(server);

			let [allGlobal, allClient, allServer] = getConfigPackages(configuration);
			// Derived packages are in the full config but not the editable one
			setDerivedGlobalPackages(
				allGlobal.filter(
					(x) => !globalPackages().some((y) => packageConfigsEqual(x, y))
				)
			);
			setDerivedClientPackages(
				allClient.filter(
					(x) => !clientPackages().some((y) => packageConfigsEqual(x, y))
				)
			);
			setDerivedServerPackages(
				allServer.filter(
					(x) => !serverPackages().some((y) => packageConfigsEqual(x, y))
				)
			);

			setPackageOverrides(
				editableConfiguration.overrides == undefined
					? {}
					: editableConfiguration.overrides
			);

			setEditableConfig(editableConfiguration);
			return configuration;
		} catch (e) {
			errorToast("Failed to load instance: " + e);
			return undefined;
		}
	});

	let [parentConfigs, __] = createResource(
		() => from(),
		async () => {
			return await getParentTemplates(from(), InstanceConfigMode.Instance);
		},
		{ initialValue: [] }
	);

	let [bannerImages, ___] = createResource(
		() => instance(),
		async () => {
			if (instance() == undefined) {
				return undefined;
			}

			try {
				let images = (await invoke("get_version_banner_images", {
					version: instance()!.version,
				})) as [string, string] | undefined;

				return images;
			} catch (e) {
				console.error("Failed to get banner images: " + e);
				return undefined;
			}
		}
	);

	let [isRunning, setIsRunning] = createSignal(false);
	let [unlisten, setUnlisten] = createSignal<UnlistenFn>(() => {});
	createEffect(async () => {
		let unlisten = await listen(
			"nitro_update_running_instances",
			(e: Event<RunningInstancesEvent>) => {
				setIsRunning(e.payload.running_instances.includes(id));
			}
		);

		setUnlisten(() => unlisten);

		await invoke("update_running_instances");
	});

	// Gets whether the currently selected instance is launchable (it has been updated before)
	let [unlisten2, setUnlisten2] = createSignal<UnlistenFn>(() => {});
	let [isInstanceLaunchable, methods] = createResource(async () => {
		let unlisten = await listen(
			"nitro_output_finish_task",
			(e: Event<string>) => {
				if (e.payload == "update_instance") {
					methods.refetch();
				}
			}
		);

		setUnlisten2(() => unlisten);

		return await invoke("get_instance_has_updated", {
			instance: id,
		});
	});

	onCleanup(() => {
		unlisten()();
		unlisten2()();
	});

	let [launchDropdownButtons, _1] = createResource(
		async () => {
			return getDropdownButtons("instance_launch");
		},
		{ initialValue: [] }
	);

	let [updateDropdownButtons, _2] = createResource(
		async () => {
			return getDropdownButtons("instance_update");
		},
		{ initialValue: [] }
	);

	let [moreDropdownButtons, _3] = createResource(
		async () => {
			return getDropdownButtons("instance_more_options");
		},
		{ initialValue: [] }
	);

	let [packageOverrides, setPackageOverrides] = createSignal<PackageOverrides>(
		{}
	);

	let [selectedTab, setSelectedTab] = createSignal("general");

	let [showDeleteConfirm, setShowDeleteConfirm] = createSignal(false);
	let [showExportPrompt, setShowExportPrompt] = createSignal(false);

	async function saveConfig() {
		if (editableConfig() != undefined) {
			let config = editableConfig()!;
			config.packages = createConfiguredPackages(
				globalPackages(),
				clientPackages(),
				serverPackages(),
				true
			);

			let overrides =
				packageOverrides().suppress == undefined
					? undefined
					: packageOverrides();
			config.overrides = overrides;

			try {
				await saveInstanceConfig(id, config, InstanceConfigMode.Instance);
				successToast("Changes saved");
				props.setFooterData({
					selectedItem: undefined,
					mode: FooterMode.SaveInstanceConfig,
					action: () => {},
				});
			} catch (e) {
				errorToast("Failed to save: " + e);
			}
		}
	}

	let setDirty = () => {
		props.setFooterData({
			selectedItem: "",
			mode: FooterMode.SaveInstanceConfig,
			action: saveConfig,
		});
	};

	let launchOptions = () => {
		let options: Option[] = [
			{
				value: "launch",
				contents: (
					<IconAndText
						icon={Play}
						size="1.25rem"
						text="Launch"
						color="var(--instance)"
					/>
				),
			},
			{
				value: "launch_offline",
				contents: (
					<IconAndText
						icon={Play}
						size="1.25rem"
						text="Launch Offline"
						color="var(--template)"
					/>
				),
			},
		];

		if (isRunning()) {
			options.push({
				value: "kill",
				contents: <IconAndText icon={Stop} text="Kill" color="var(--error)" />,
				backgroundColor: "var(--errorbg)",
			});
		}

		options.concat(launchDropdownButtons().map(dropdownButtonToOption));

		return options;
	};

	return (
		<Show
			when={instance() != undefined}
			fallback={
				<div class="cont" style="width:100%">
					<LoadingSpinner size="5rem" />
				</div>
			}
		>
			<div class="cont col fullwidth">
				<div class="cont col" id="instance-container">
					<div class="cont" id="instance-header-container">
						<div class="shadow" id="instance-header">
							<div class="cont start" id="instance-icon">
								<img
									id="instance-icon-image"
									src={
										instance()!.icon == undefined
											? "/icons/default_instance.png"
											: getInstanceIconSrc(instance()!.icon)
									}
									onerror={(e) =>
										((e.target as any).src = "/icons/default_instance.png")
									}
								/>
							</div>
							<div id="instance-details-container">
								<div class="col" id="instance-details">
									<div class="cont" id="instance-upper-details">
										<div id="instance-name">
											{instance()!.name == undefined ? id : instance()!.name}
										</div>
										<Show when={instance()!.name != undefined}>
											<div id="instance-id">{id}</div>
										</Show>
									</div>
									<div class="cont start" id="instance-lower-details">
										<div class="cont start" id="instance-version">
											<Icon icon={Tag} size="0.75rem" />
											{instance()!.version}
										</div>
										<PackageLabels
											categories={[]}
											loaders={
												instance()!.loader == undefined
													? []
													: [instance()!.loader! as string]
											}
											packageTypes={[]}
										/>
									</div>
								</div>
								<div class="cont end" style="margin-right:1rem">
									<Show when={isInstanceLaunchable()}>
										<div style="width:9rem;font-weight:bold">
											<Dropdown
												options={launchOptions()}
												previewText={
													<Switch>
														<Match when={!isRunning()}>
															<IconAndText
																icon={Play}
																size="1.25rem"
																text="Launch"
																centered
															/>
														</Match>
														<Match when={isRunning()}>
															<IconAndText
																icon={Stop}
																text="Kill"
																color="var(--error)"
																centered
															/>
														</Match>
													</Switch>
												}
												onChange={async (selection) => {
													if (selection == "launch") {
														launchInstance(id, false);
													} else if (selection == "launch_offline") {
														launchInstance(id, true);
													} else if (selection == "kill") {
														try {
															await invoke("kill_instance", { instance: id });
															await invoke("update_running_instances");
														} catch (e) {
															errorToast("Failed to kill instance: " + e);
														}
													} else {
														runDropdownButtonClick(selection!);
													}
												}}
												onHeaderClick={async () => {
													if (isRunning()) {
														try {
															await invoke("kill_instance", { instance: id });
															await invoke("update_running_instances");
														} catch (e) {
															errorToast("Failed to kill instance: " + e);
														}
													} else {
														launchInstance(id, false);
													}
												}}
												optionsWidth="12rem"
												isSearchable={false}
												zIndex="5"
											/>
										</div>
									</Show>
									<IconTextButton
										icon={Gear}
										size="1.2rem"
										text="Configure"
										onClick={() => {
											navigate(`/instance_config/${id}`);
										}}
									/>
									<div style="width:9rem;font-weight:bold">
										<Dropdown
											options={(
												[
													{
														value: "update",
														contents: (
															<IconAndText icon={Upload} text="Update" />
														),
														tip: "Update the packages and files on this instance",
													},
													{
														value: "force_update",
														contents: (
															<IconAndText
																icon={Upload}
																text="Force Update"
																color="var(--error)"
															/>
														),
														backgroundColor: "var(--errorbg)",
														tip: "Update while replacing already cached files. Should only be done if something is broken.",
													},
												] as Option[]
											).concat(
												updateDropdownButtons().map(dropdownButtonToOption)
											)}
											previewText={
												<IconAndText icon={Upload} text="Update" centered />
											}
											onChange={async (selection) => {
												if (
													selection == "update" ||
													selection == "force_update"
												) {
													try {
														let depth =
															selection == "update" ? "full" : "force";

														await invoke("update_instance", {
															instanceId: id,
															depth: depth,
														});
													} catch (e) {
														errorToast("Failed to update instance: " + e);
													}
												} else {
													runDropdownButtonClick(selection!);
												}
											}}
											onHeaderClick={async () => {
												try {
													await invoke("update_instance", {
														instanceId: id,
														depth: "full",
													});
												} catch (e) {
													errorToast("Failed to update instance: " + e);
												}
											}}
											optionsWidth="12rem"
											isSearchable={false}
											zIndex="5"
										/>
									</div>
									<div style="width:9rem;font-weight:bold">
										<Dropdown
											options={(
												[
													{
														value: "export",
														contents: (
															<IconAndText icon={Popout} text="Export" />
														),
														tip: "Export this instance to another launcher",
													},
													{
														value: "open_dir",
														contents: (
															<IconAndText icon={Folder} text="Open Folder" />
														),
														tip: "Open this instance's files in your explorer",
													},
													{
														value: "delete",
														contents: (
															<IconAndText
																icon={Trash}
																text="Delete"
																color="var(--error)"
															/>
														),
														tip: "Delete this instance forever",
														backgroundColor: "var(--errorbg)",
													},
												] as Option[]
											).concat(
												moreDropdownButtons().map(dropdownButtonToOption)
											)}
											previewText={
												<IconAndText icon={Elipsis} text="More" centered />
											}
											onChange={async (selection) => {
												if (selection == "export") {
													setShowExportPrompt(true);
												} else if (selection == "open_dir") {
													await invoke("open_instance_dir", { instance: id });
												} else if (selection == "delete") {
													setShowDeleteConfirm(true);
												} else {
													runDropdownButtonClick(selection!);
												}
											}}
											optionsWidth="11rem"
											isSearchable={false}
											zIndex="5"
										/>
									</div>
								</div>
							</div>
						</div>
					</div>
					<Show when={bannerImages() != undefined}>
						<div id="instance-banner-container">
							<div id="instance-banner">
								<img
									src={convertFileSrc(bannerImages()![0])}
									onerror={(e) => e.target.remove()}
								/>
								<img
									src={convertFileSrc(bannerImages()![1])}
									onerror={(e) => e.target.remove()}
								/>
							</div>
							<div id="instance-banner-gradient"></div>
						</div>
					</Show>
					<div id="instance-contents">
						<div id="instance-body">
							<div
								class="instance-shadow"
								id="instance-tabs"
								style={`grid-template-columns:repeat(3,minmax(0,1fr))`}
							>
								<div
									class={`cont instance-tab ${
										selectedTab() == "general" ? "selected" : ""
									}`}
									onclick={() => setSelectedTab("general")}
								>
									<Icon icon={Gear} size="1rem" />
									General
								</div>
								<div
									class={`cont instance-tab ${
										selectedTab() == "packages" ? "selected" : ""
									}`}
									onclick={() => setSelectedTab("packages")}
								>
									<Icon icon={Box} size="1rem" />
									Packages
								</div>
								<div
									class={`cont instance-tab ${
										selectedTab() == "console" ? "selected" : ""
									}`}
									onclick={() => setSelectedTab("console")}
								>
									<Icon icon={Text} size="1rem" />
									Console
								</div>
							</div>
							<div class="cont col instance-shadow" id="instance-tab-contents">
								<Show when={selectedTab() == "general"}>
									<InstanceTiles instanceId={id} />
								</Show>
								<Show when={selectedTab() == "packages"}>
									<PackagesConfig
										id={id}
										globalPackages={globalPackages()}
										clientPackages={clientPackages()}
										serverPackages={serverPackages()}
										derivedGlobalPackages={derivedGlobalPackages()}
										derivedClientPackages={derivedClientPackages()}
										derivedServerPackages={derivedServerPackages()}
										isTemplate={false}
										onRemove={(pkg, category) => {
											let func = (packages: PackageConfig[]) =>
												packages.filter((x) => !packageConfigsEqual(x, pkg));

											if (category == "global") {
												setGlobalPackages(func);
											} else if (category == "client") {
												setClientPackages(func);
											} else if (category == "server") {
												setServerPackages(func);
											}

											setDirty();
										}}
										onAdd={(pkg, category) => {
											let func = (packages: PackageConfig[]) => {
												if (
													!packages.some((x) =>
														packageConfigsFullyEqual(x, pkg)
													)
												) {
													packages.push(pkg);
													// Force update
													packages = packages.concat([]);
												}
												return packages;
											};

											if (category == "global") {
												setGlobalPackages(func);
											} else if (category == "client") {
												setClientPackages(func);
											} else if (category == "server") {
												setServerPackages(func);
											}

											setDirty();
										}}
										setGlobalPackages={() => {}}
										setClientPackages={() => {}}
										setServerPackages={() => {}}
										minecraftVersion={instance()!.version}
										loader={
											parseVersionedString(
												instance()!.loader as string
											)[0] as Loader
										}
										showBrowseButton={true}
										parentConfigs={parentConfigs()}
										onChange={setDirty}
										overrides={packageOverrides()}
										setOverrides={setPackageOverrides}
										beforeUpdate={saveConfig}
									/>
								</Show>
								<Show when={selectedTab() == "console"}>
									<div class="cont" style="width: 100%">
										<InstanceConsole
											instanceId={id}
											isServer={
												instance() != undefined && instance()!.type == "server"
											}
										/>
									</div>
								</Show>
							</div>
						</div>
					</div>
				</div>
				<Modal
					visible={showDeleteConfirm()}
					onClose={setShowDeleteConfirm}
					title="Delete instance"
					titleIcon={Trash}
					buttons={[
						{
							text: "Cancel",
							icon: Delete,
							color: "var(--instance)",
							bgColor: "var(--instancebg)",
							onClick: () => setShowDeleteConfirm(false),
						},
						{
							text: "Delete instance",
							icon: Trash,
							color: "var(--fg3)",
							onClick: async () => {
								try {
									await invoke("delete_instance", { instance: id });
									successToast("Instance deleted");
									setShowDeleteConfirm(false);
									updateInstanceList();
									navigate("/");
								} catch (e) {
									errorToast("Failed to delete instance: " + e);
									setShowDeleteConfirm(false);
								}
							},
						},
					]}
				>
					<h3>Are you sure you want to delete this instance?</h3>
					<div class="cont bold" style="font-size:0.9rem;color:var(--fg2)">
						This will delete ALL of your worlds and data for the instance!
					</div>
				</Modal>
				<InstanceTransferPrompt
					visible={showExportPrompt()}
					onClose={() => setShowExportPrompt(false)}
					exportedInstance={id}
				/>
				<br />
				<br />
				<br />
			</div>
		</Show>
	);
}

export interface InstanceInfoProps {
	setFooterData: (data: FooterData) => void;
}
