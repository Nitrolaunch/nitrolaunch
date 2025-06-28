import { useParams } from "@solidjs/router";
import { createResource, createSignal, onMount, Show } from "solid-js";
import { loadPagePlugins } from "../../plugins";
import { InstanceConfigMode, readInstanceConfig } from "./read_write";
import { errorToast } from "../../components/dialog/Toasts";
import LoadingSpinner from "../../components/utility/LoadingSpinner";
import { getInstanceIconSrc } from "../../utils";
import PackageLabels from "../../components/package/PackageLabels";
import { Loader } from "../../package";
import Icon from "../../components/Icon";
import { Gear, Play, Text, Upload } from "../../icons";
import "./InstanceInfo.css";
import IconTextButton from "../../components/input/IconTextButton";
import { invoke } from "@tauri-apps/api";
import InstanceConsole from "../../components/launch/InstanceConsole";

export default function InstanceInfo() {
	let params = useParams();
	let id = params.instanceId;

	onMount(() => loadPagePlugins("instance", id));

	let [instance, _] = createResource(async () => {
		// Get the instance or profile
		try {
			let configuration = await readInstanceConfig(
				id,
				InstanceConfigMode.Instance
			);
			return configuration;
		} catch (e) {
			errorToast("Failed to load instance: " + e);
			return undefined;
		}
	});

	let [selectedTab, setSelectedTab] = createSignal("general");

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
								style={`grid-template-columns:repeat(2,minmax(0,1fr))`}
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
