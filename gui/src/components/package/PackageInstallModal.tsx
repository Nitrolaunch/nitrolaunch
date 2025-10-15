import { createResource, createSignal, Match, Show, Switch } from "solid-js";
import Modal from "../dialog/Modal";
import "./PackageInstallModal.css";
import IconTextButton from "../input/button/IconTextButton";
import {
	AngleRight,
	Box,
	Controller,
	Delete,
	Diagram,
	Download,
	Folder,
	Globe,
	Hashtag,
	Server,
} from "../../icons";
import Icon from "../Icon";
import InlineSelect from "../input/select/InlineSelect";
import { InstanceInfo, InstanceOrProfile } from "../../types";
import { invoke } from "@tauri-apps/api";
import { errorToast, successToast } from "../dialog/Toasts";
import {
	addPackage,
	InstanceConfigMode,
	readEditableInstanceConfig,
	saveInstanceConfig,
} from "../../pages/instance/read_write";
import { pkgRequestToString } from "../../utils";

export default function PackageInstallModal(props: PackageInstallModalProps) {
	let [selectedTab, setSelectedTab] = createSignal("instance");

	let [selectedInstanceOrProfile, setSelectedInstanceOrProfile] = createSignal<
		string | undefined
	>(undefined);
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

	// Automatically set the type and id based on what the user last added a package to
	createResource(async () => {
		let lastAdded = (await invoke("get_last_added_package_location")) as
			| [string, InstanceOrProfile]
			| undefined;
		if (lastAdded != undefined) {
			// Don't overwrite if the user already selected
			if (selectedInstanceOrProfile() == undefined) {
				setSelectedTab(lastAdded[1]);
				setSelectedInstanceOrProfile(lastAdded[0]);
			}
		}
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
			let config = await readEditableInstanceConfig(
				selectedInstanceOrProfile(),
				mode
			);
			let pkg = pkgRequestToString({
				id: props.packageId,
				version: props.selectedVersion,
				repository: props.packageRepo,
			});

			addPackage(config, pkg, location);

			await saveInstanceConfig(selectedInstanceOrProfile(), config, mode);

			// Save the last added location
			invoke("set_last_added_package_location", {
				id: selectedInstanceOrProfile(),
				instanceOrProfile: selectedTab(),
			});

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
										contents: <div class="cont"><Icon icon={Controller} size="1.2rem" /> Instance</div>,
										color: "var(--instance)",
									},
									{
										value: "profile",
										contents: <div class="cont"><Icon icon={Server} size="1.2rem" /> Profile</div>,
										color: "var(--profile)",
									},
									{
										value: "global_profile",
										contents: <div class="cont"><Icon icon={Globe} size="1rem" />  Globally</div>,
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
						<div class="cont fullwidth" id="package-install-profile-location">
							<div style="width:40%">
								What children of this profile should get this package?
							</div>
							<div class="cont" style="width:60%">
								<InlineSelect
									options={[
										{
											value: "all",
											contents: <div class="cont"><Icon icon={Globe} size="1rem" />  All of them</div>,
											color: "var(--fg2)",
										},
										{
											value: "client",
											contents: <div class="cont"><Icon icon={Controller} size="1.2rem" /> Clients</div>,
											color: "var(--instance)",
										},
										{
											value: "server",
											contents: <div class="cont"><Icon icon={Server} size="1rem" /> Servers</div>,
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
						onClick={() => {
							props.onClose();
						}}
						text="Close"
					/>
					<IconTextButton
						icon={Download}
						size="1.5rem"
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
