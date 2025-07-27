import { useParams } from "@solidjs/router";
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
import { loadPagePlugins } from "../../plugins";
import {
	createConfiguredPackages,
	getConfigPackages,
	getParentProfiles,
	InstanceConfig,
	InstanceConfigMode,
	PackageOverrides,
	readEditableInstanceConfig,
	readInstanceConfig,
	saveInstanceConfig,
} from "./read_write";
import { errorToast, successToast } from "../../components/dialog/Toasts";
import LoadingSpinner from "../../components/utility/LoadingSpinner";
import { getInstanceIconSrc } from "../../utils";
import PackageLabels from "../../components/package/PackageLabels";
import { Loader } from "../../package";
import Icon from "../../components/Icon";
import { Box, Delete, Gear, Play, Spinner, Text, Upload } from "../../icons";
import "./InstanceInfo.css";
import IconTextButton from "../../components/input/IconTextButton";
import { invoke } from "@tauri-apps/api";
import InstanceConsole from "../../components/launch/InstanceConsole";
import PackagesConfig, {
	getPackageConfigRequest,
	PackageConfig,
	packageConfigsEqual,
} from "./PackagesConfig";
import { FooterData } from "../../App";
import { FooterMode, launchInstance } from "../../components/navigation/Footer";
import Modal from "../../components/dialog/Modal";
import { canonicalizeListOrSingle } from "../../utils/values";
import { Event, listen, UnlistenFn } from "@tauri-apps/api/event";
import { RunningInstancesEvent } from "../../components/launch/RunningInstanceList";

export default function InstanceInfo(props: InstanceInfoProps) {
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
				instanceOrProfile: "instance",
			});
		} catch (e) {}
	});

	let [from, setFrom] = createSignal<string[] | undefined>();
	let [editableConfig, setEditableConfig] = createSignal<InstanceConfig>();
	let [instance, _] = createResource(async () => {
		// Get the instance or profile
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
			return await getParentProfiles(from(), InstanceConfigMode.Instance);
		},
		{ initialValue: [] }
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

	let [launchButtonHovered, setLaunchButtonHovered] = createSignal(false);

	let [packageOverrides, setPackageOverrides] = createSignal<PackageOverrides>(
		{}
	);

	let [selectedTab, setSelectedTab] = createSignal("general");

	let [showDeleteConfirm, setShowDeleteConfirm] = createSignal(false);

	let setDirty = () => {
		props.setFooterData({
			selectedItem: "",
			mode: FooterMode.SaveInstanceConfig,
			action: async () => {
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
			},
		});
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
			<div class="cont col" style="width:100%">
				<div class="cont col" id="instance-container">
					<div class="cont" id="instance-header-container">
						<div class="input-shadow" id="instance-header">
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
										<PackageLabels
											categories={[]}
											loaders={
												instance()!.loader == undefined
													? []
													: [instance()!.loader! as Loader]
											}
											packageTypes={[]}
										/>
									</div>
								</div>
								<div class="cont end" style="margin-right:1rem">
									<Show when={isInstanceLaunchable()}>
										<div
											onmouseenter={() => setLaunchButtonHovered(true)}
											onmouseleave={() => setLaunchButtonHovered(false)}
											style="width:10rem"
										>
											<Switch>
												<Match when={!isRunning()}>
													<IconTextButton
														icon={Play}
														size="1.2rem"
														text="Launch"
														color="var(--bg2)"
														selected={false}
														selectedColor="var(--instance)"
														onClick={() => {
															launchInstance(id);
														}}
														shadow={false}
														style="width:100%"
													/>
												</Match>
												<Match when={isRunning() && !launchButtonHovered()}>
													<IconTextButton
														icon={Spinner}
														size="1.2rem"
														text="Running"
														color={"var(--bg2)"}
														selected={false}
														selectedColor="var(--instance)"
														onClick={() => {}}
														shadow={false}
														style="width:100%"
														animate
													/>
												</Match>
												<Match when={isRunning() && launchButtonHovered()}>
													<IconTextButton
														icon={Delete}
														size="1.2rem"
														text="Kill"
														color="var(--errorbg)"
														selected={true}
														selectedColor="var(--error)"
														onClick={async () => {
															await invoke("kill_instance", { instance: id });
															await invoke("update_running_instances");
														}}
														shadow={false}
														style="width:100%"
													/>
												</Match>
											</Switch>
										</div>
									</Show>
									<IconTextButton
										icon={Gear}
										size="1.2rem"
										text="Configure"
										color="var(--bg2)"
										selected={false}
										selectedColor="var(--instance)"
										onClick={() => {
											window.location.href = `/instance_config/${id}`;
										}}
										shadow={false}
									/>
									<IconTextButton
										icon={Upload}
										size="1.2rem"
										text="Update"
										color="var(--bg2)"
										selected={false}
										selectedColor="var(--profile)"
										onClick={async () => {
											try {
												await invoke("update_instance", {
													instanceId: id,
												});
											} catch (e) {
												errorToast("Failed to update instance: " + e);
											}
										}}
										shadow={false}
									/>
									<IconTextButton
										icon={Delete}
										size="1.2rem"
										text="Delete"
										color="var(--errorbg)"
										selectedColor="var(--error)"
										selectedBg="var(--errorbg)"
										selected={true}
										onClick={() => setShowDeleteConfirm(true)}
										shadow={false}
									/>
								</div>
							</div>
						</div>
					</div>
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
								<Show when={selectedTab() == "general"}>{""}</Show>
								<Show when={selectedTab() == "packages"}>
									<PackagesConfig
										id={id}
										globalPackages={globalPackages()}
										clientPackages={clientPackages()}
										serverPackages={serverPackages()}
										derivedGlobalPackages={derivedGlobalPackages()}
										derivedClientPackages={derivedClientPackages()}
										derivedServerPackages={derivedServerPackages()}
										isProfile={false}
										onRemove={(pkg, category) => {
											if (category == "global") {
												setGlobalPackages((packages) =>
													packages.filter(
														(x) => getPackageConfigRequest(x).id != pkg
													)
												);
											} else if (category == "client") {
												setClientPackages((packages) =>
													packages.filter(
														(x) => getPackageConfigRequest(x).id != pkg
													)
												);
											} else if (category == "server") {
												setServerPackages((packages) =>
													packages.filter(
														(x) => getPackageConfigRequest(x).id != pkg
													)
												);
											}

											setDirty();
										}}
										setGlobalPackages={() => {}}
										setClientPackages={() => {}}
										setServerPackages={() => {}}
										minecraftVersion={instance()!.version}
										loader={instance()!.loader as Loader}
										showBrowseButton={true}
										parentConfigs={parentConfigs()}
										onChange={setDirty}
										overrides={packageOverrides()}
										setOverrides={setPackageOverrides}
									/>
								</Show>
								<Show when={selectedTab() == "console"}>
									<div class="cont" style="width: 100%">
										<InstanceConsole instanceId={id} />
									</div>
								</Show>
							</div>
						</div>
					</div>
				</div>
				<Modal
					visible={showDeleteConfirm()}
					onClose={setShowDeleteConfirm}
					width="25rem"
				>
					<div class="cont col" style="padding:2rem">
						<h3>Are you sure you want to delete this instance?</h3>
						<div class="cont bold" style="font-size:0.9rem;color:var(--fg2)">
							This will delete ALL of your worlds and data for the instance!
						</div>
						<div></div>
						<div></div>
						<div class="cont">
							<button
								onclick={() => setShowDeleteConfirm(false)}
								style="border-color:var(--instance)"
							>
								Cancel
							</button>
							<IconTextButton
								icon={Delete}
								size="1rem"
								text="Delete instance"
								color="var(--errorbg)"
								selectedColor="var(--error)"
								selectedBg="var(--errorbg)"
								selected={true}
								onClick={async () => {
									try {
										await invoke("delete_instance", { instance: id });
										successToast("Instance deleted");
										setShowDeleteConfirm(false);
										window.location.href = "/";
									} catch (e) {
										errorToast("Failed to delete instance: " + e);
										setShowDeleteConfirm(false);
									}
								}}
							/>
						</div>
					</div>
				</Modal>
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
