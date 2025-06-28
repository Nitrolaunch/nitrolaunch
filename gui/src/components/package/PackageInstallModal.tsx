import { createResource, createSignal, Match, Show, Switch } from "solid-js";
import Modal from "../dialog/Modal";
import "./PackageInstallModal.css";
import IconTextButton from "../input/IconTextButton";
import {
	AngleRight,
	Box,
	Delete,
	Diagram,
	Download,
	Folder,
	Hashtag,
} from "../../icons";
import Icon from "../Icon";
import InlineSelect from "../input/InlineSelect";
import { InstanceInfo } from "../../types";
import { invoke } from "@tauri-apps/api";
import { errorToast, successToast } from "../dialog/Toasts";
import {
	addPackage,
	InstanceConfigMode,
	readInstanceConfig,
	saveInstanceConfig,
} from "../../pages/instance/read_write";
import { pkgRequestToString } from "../../utils";

export default function PackageInstallModal(props: PackageInstallModalProps) {
	let [selectedTab, setSelectedTab] = createSignal("instance");

	let [selectedInstanceOrProfile, setSelectedInstanceOrProfile] =
		createSignal(undefined);
	let [selectedProfileLocation, setSelectedProfileLocation] =
		createSignal("all");

	let [instancesAndProfiles, _] = createResource(async () => {
		let instances: InstanceInfo[] = [];
		let profiles: InstanceInfo[] = [];
		try {
			[instances, profiles] = (await Promise.all([
				invoke("get_instances"),
				invoke("get_profiles"),
			])) as [InstanceInfo[], InstanceInfo[]];
		} catch (e) {
			errorToast("Failed to get instances and profiles: " + e);
		}

		return [instances, profiles];
	});

	let install = async () => {
		if (
			selectedTab() != "global_profile" &&
			selectedInstanceOrProfile() == undefined
		) {
			return;
		}

		let mode = selectedTab() as InstanceConfigMode;
		let location =
			selectedProfileLocation() == undefined ||
			mode == InstanceConfigMode.Instance
				? "all"
				: (selectedProfileLocation() as "client" | "server" | "all");

		try {
			let config = await readInstanceConfig(selectedInstanceOrProfile(), mode);
			let pkg = pkgRequestToString({
				id: props.packageId,
				version: props.selectedVersion,
				repo: props.packageRepo,
			});

			addPackage(config, pkg, location);

			await saveInstanceConfig(selectedInstanceOrProfile(), config, mode);

			successToast(
				mode == InstanceConfigMode.Instance
					? "Package added. Remember to update the instance to use it"
					: "Package added"
			);
			props.onClose();
		} catch (e) {
			errorToast("Failed to add package: " + e);
		}
	};

	return (
		<Modal visible={props.visible} onClose={props.onClose} width="55rem">
			<div id="package-install">
				<div id="package-install-inner">
					<div class="cont" style="margin-bottom:1rem">
						<Icon icon={Download} size="1.2rem" />
					</div>
					<div class="cont" id="package-install-name">
						Installing package
						<div style="color:var(--fg3)">{props.packageId}</div>
					</div>
					<Switch>
						<Match when={props.selectedVersion == undefined}>
							<div class="cont">
								<Icon icon={Hashtag} size="1.2rem" />
							</div>
							<div class="cont" id="package-install-version">
								<div style="width:68%">
									No version selected. The best version will be picked
									automatically.
								</div>
								<div class="cont" style="width:32%;justify-content:flex-end">
									<IconTextButton
										icon={AngleRight}
										size="1.5rem"
										color="var(--bg2)"
										selectedColor="var(--bg2)"
										selected={false}
										onClick={() => {
											props.onClose();
											props.onShowVersions();
										}}
										text="Select a version"
									/>
								</div>
							</div>
						</Match>
						<Match when={props.selectedVersion != undefined}>
							<div class="cont">
								<Icon icon={Hashtag} size="1.2rem" />
							</div>
							<div class="cont" id="package-install-version">
								<div>Selected version {props.selectedVersion}</div>
							</div>
						</Match>
					</Switch>
					<div class="cont">
						<Icon icon={Folder} size="1.2rem" />
					</div>
					<div class="cont" id="package-install-target-category">
						<div style="width:40%">Where would you like to install?</div>
						<div class="cont" style="width:60%">
							<InlineSelect
								options={[
									{
										value: "instance",
										contents: <div>Instance</div>,
										color: "var(--instance)",
									},
									{
										value: "profile",
										contents: <div>Profile</div>,
										color: "var(--profile)",
									},
									{
										value: "global_profile",
										contents: <div>Globally</div>,
										color: "var(--pluginfg)",
									},
								]}
								selected={selectedTab()}
								onChange={(tab) => {
									setSelectedTab(tab!);
									setSelectedInstanceOrProfile(undefined);
								}}
							/>
						</div>
					</div>
					<Show when={selectedTab() != "global_profile"}>
						<div class="cont">
							<Icon icon={Box} size="1.2rem" />
						</div>
						<div class="cont" id="package-install-target">
							<div>Select {selectedTab()}</div>
						</div>
						<div></div>
						<div class="cont" style="width:100%">
							<Show when={instancesAndProfiles() != undefined}>
								<InlineSelect
									options={(selectedTab() == "instance"
										? instancesAndProfiles()![0]
										: instancesAndProfiles()![1]
									).map((item) => {
										return {
											value: item.id,
											contents: (
												<div>
													{item.name == undefined ? item.id : item.name}
												</div>
											),
											color: `var(--${selectedTab()})`,
										};
									})}
									selected={selectedInstanceOrProfile()}
									onChange={setSelectedInstanceOrProfile}
									columns={4}
									connected={false}
								/>
							</Show>
						</div>
					</Show>
					<Show when={selectedTab() == "profile"}>
						<div class="cont">
							<Icon icon={Diagram} size="1.2rem" />
						</div>
						<div class="cont" id="package-install-profile-location">
							<div style="width:40%">
								What children of this profile should get this package?
							</div>
							<div class="cont" style="width:60%">
								<InlineSelect
									options={[
										{
											value: "all",
											contents: <div>All of them</div>,
											color: "var(--fg2)",
										},
										{
											value: "client",
											contents: <div>Clients</div>,
											color: "var(--instance)",
										},
										{
											value: "server",
											contents: <div>Servers</div>,
											color: "var(--profile)",
										},
									]}
									selected={selectedProfileLocation()}
									onChange={setSelectedProfileLocation}
								/>
							</div>
						</div>
					</Show>
				</div>
				<br />
				<br />
				<div class="cont" style="width:100%">
					<IconTextButton
						icon={Delete}
						size="1.5rem"
						color="var(--bg2)"
						selectedColor="var(--package)"
						selectedBg="var(--bg)"
						selected={false}
						onClick={() => {
							props.onClose();
						}}
						text="Close"
					/>
					<IconTextButton
						icon={Download}
						size="1.5rem"
						color="var(--bg2)"
						selectedColor="var(--package)"
						selectedBg="var(--bg)"
						selected={false}
						onClick={() => {
							install();
						}}
						text="Install"
					/>
				</div>
			</div>
		</Modal>
	);
}

export interface PackageInstallModalProps {
	packageId: string;
	packageRepo?: string;
	selectedVersion?: string;
	visible: boolean;
	onClose: () => void;
	// Function to show the versions tab
	onShowVersions: () => void;
}
