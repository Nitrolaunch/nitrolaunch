import { useParams } from "@solidjs/router";
import { createResource, createSignal, onMount, Show } from "solid-js";
import { loadPagePlugins } from "../../plugins";
import {
	createConfiguredPackages,
	getConfigPackages,
	InstanceConfigMode,
	readInstanceConfig,
	saveInstanceConfig,
} from "./read_write";
import { errorToast, successToast } from "../../components/dialog/Toasts";
import LoadingSpinner from "../../components/utility/LoadingSpinner";
import { getInstanceIconSrc } from "../../utils";
import PackageLabels from "../../components/package/PackageLabels";
import { Loader } from "../../package";
import Icon from "../../components/Icon";
import { Box, Gear, Play, Text, Upload } from "../../icons";
import "./InstanceInfo.css";
import IconTextButton from "../../components/input/IconTextButton";
import { invoke } from "@tauri-apps/api";
import InstanceConsole from "../../components/launch/InstanceConsole";
import PackagesConfig, {
	getPackageConfigRequest,
	PackageConfig,
} from "./PackagesConfig";
import { FooterData } from "../../App";
import { FooterMode } from "../../components/navigation/Footer";

export default function InstanceInfo(props: InstanceInfoProps) {
	let params = useParams();
	let id = params.instanceId;

	onMount(() => loadPagePlugins("instance", id));

	// Global, client, and server packages for the instance
	let [globalPackages, setGlobalPackages] = createSignal<PackageConfig[]>([]);
	let [clientPackages, setClientPackages] = createSignal<PackageConfig[]>([]);
	let [serverPackages, setServerPackages] = createSignal<PackageConfig[]>([]);

	let [instance, _] = createResource(async () => {
		// Get the instance or profile
		try {
			let configuration = await readInstanceConfig(
				id,
				InstanceConfigMode.Instance
			);

			let [global, client, server] = getConfigPackages(configuration);
			setGlobalPackages(global);
			setClientPackages(client);
			setServerPackages(server);

			return configuration;
		} catch (e) {
			errorToast("Failed to load instance: " + e);
			return undefined;
		}
	});

	let [selectedTab, setSelectedTab] = createSignal("general");

	let setDirty = () => {
		props.setFooterData({
			selectedItem: "",
			mode: FooterMode.SaveInstanceConfig,
			action: async () => {
				if (instance() != undefined) {
					let config = instance()!;
					config.packages = createConfiguredPackages(
						globalPackages(),
						clientPackages(),
						serverPackages(),
						true
					);
					try {
						await saveInstanceConfig(
							id,
							instance()!,
							InstanceConfigMode.Instance
						);
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
									<IconTextButton
										icon={Play}
										size="1.2rem"
										text="Launch"
										color="var(--bg2)"
										selected={false}
										selectedColor="var(--instance)"
										onClick={() => {}}
										shadow={false}
									/>
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
										globalPackages={globalPackages()!}
										clientPackages={clientPackages()!}
										serverPackages={serverPackages()!}
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
