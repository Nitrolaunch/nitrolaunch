import { createMemo, createResource, createSignal, Match, Show, Switch } from "solid-js";
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
	Plus,
	Server,
} from "../../icons";
import Icon from "../Icon";
import InlineSelect, { Option } from "../input/select/InlineSelect";
import { InstanceInfo, InstanceOrTemplate } from "../../types";
import { invoke } from "@tauri-apps/api/core";
import { errorToast, successToast } from "../dialog/Toasts";
import {
	addPackage,
	InstanceConfigMode,
	readEditableInstanceConfig,
	saveInstanceConfig,
} from "../../pages/instance/read_write";
import { pkgRequestToString } from "../../utils";
import Modal from "../dialog/Modal";
import { emptyUndefined } from "../../utils/values";
import { clearInputError, inputError } from "../../errors";

export default function PackageInstallModal(props: PackageInstallModalProps) {
	let [selectedTab, setSelectedTab] = createSignal("instance");

	let [selectedInstanceOrTemplate, setSelectedInstanceOrTemplate] =
		createSignal<string | undefined>(undefined);
	let [selectedTemplateLocation, setSelectedTemplateLocation] =
		createSignal("all");
	let [newInstanceId, setNewInstanceId] = createSignal<string | undefined>();

	let [instancesAndTemplates, _] = createResource(async () => {
		let instances: InstanceInfo[] = [];
		let templates: InstanceInfo[] = [];
		try {
			[instances, templates] = (await Promise.all([
				invoke("get_instances"),
				invoke("get_templates"),
			])) as [InstanceInfo[], InstanceInfo[]];
		} catch (e) {
			errorToast("Failed to get instances and templates: " + e);
		}

		return [instances, templates];
	});

	let name = createMemo(() => {
		if (props.packageName != undefined) {
			return props.packageName;
		} else if (props.packageSlug != undefined) {
			return props.packageSlug;
		} else {
			return props.packageId;
		}
	});

	let isModpack = () => props.modpackMinecraftVersions != undefined;

	// Automatically set the type and id based on what the user last added a package to
	createResource(async () => {
		let lastAdded = (await invoke("get_last_added_package_location")) as
			| [string, InstanceOrTemplate]
			| undefined;
		if (lastAdded != undefined) {
			// Don't overwrite if the user already selected
			if (selectedInstanceOrTemplate() == undefined) {
				setSelectedTab(lastAdded[1]);
				setSelectedInstanceOrTemplate(lastAdded[0]);
			}
		}
	});

	let installLocationOptions = createMemo(() => {
		let options: Option[] = [];

		let instanceOptionName = isModpack() ? "Existing Instance" : "Instance";
		options.push({
			value: "instance",
			contents: (
				<div class="cont">
					<Icon icon={Box} size="1rem" /> {instanceOptionName}
				</div>
			),
			color: "var(--instance)",
			selectedBgColor: "var(--instancebg)",
		});

		options.push({
			value: "template",
			contents: (
				<div class="cont">
					<Icon icon={Diagram} size="1rem" /> Template
				</div>
			),
			color: "var(--template)",
			selectedBgColor: "var(--templatebg)",
		});

		if (isModpack()) {
			options.push({
				value: "new_instance",
				contents: (
					<div class="cont">
						<Icon icon={Plus} size="1rem" /> New Instance
					</div>
				),
				color: "var(--instance)",
				selectedBgColor: "var(--instancebg)",
			});
		} else {
			options.push({
				value: "base_template",
				contents: (
					<div class="cont">
						<Icon icon={Globe} size="1rem" /> Globally
					</div>
				),
				color: "var(--template)",
				selectedBgColor: "var(--templatebg)",
			});
		}

		return options;
	});

	let installModpack = async () => {
		let pkg = pkgRequestToString({
			id: props.packageId,
			slug: props.packageSlug,
			version: props.selectedVersion,
			repository: props.packageRepo,
		});

		if (selectedTab() == "instance" || selectedTab() == "template") {
			if (selectedInstanceOrTemplate() == undefined) {
				return;
			}

			let mode = selectedTab() as InstanceConfigMode;
			let [config, _] = await readEditableInstanceConfig(
				selectedInstanceOrTemplate(),
				mode,
			);

			config.modpack = pkg;

			successToast("Modpack added");
		} else {
			if (newInstanceId() == undefined) {
				inputError("package-install-instance-id");
				return;
			} else {
				clearInputError("package-install-instance-id");
			}

			await invoke("install_modpack_package", {
				modpack: pkg,
				instanceId: newInstanceId(),
			});

			successToast("Modpack imported");
		}
	}

	let install = async () => {
		if (isModpack()) {
			try {
				await installModpack();

				props.onClose();
			} catch (e) {
				errorToast("Failed to install modpack: " + e);
			}

			return;
		}

		if (
			selectedTab() != "base_template" &&
			selectedInstanceOrTemplate() == undefined
		) {
			return;
		}

		let mode = selectedTab() as InstanceConfigMode;
		let location =
			selectedTemplateLocation() == undefined ||
				mode == InstanceConfigMode.Instance
				? "all"
				: (selectedTemplateLocation() as "client" | "server" | "all");

		try {
			let [config, _] = await readEditableInstanceConfig(
				selectedInstanceOrTemplate(),
				mode,
			);
			let pkg = pkgRequestToString({
				id: props.packageId,
				slug: props.packageSlug,
				version: props.selectedVersion,
				repository: props.packageRepo,
			});

			addPackage(config, pkg, location);

			await saveInstanceConfig(selectedInstanceOrTemplate(), config, mode);

			// Save the last added location
			invoke("set_last_added_package_location", {
				id: selectedInstanceOrTemplate(),
				instanceOrTemplate: selectedTab(),
			});

			successToast(
				mode == InstanceConfigMode.Instance
					? "Package added. Remember to update the instance to use it"
					: "Package added",
			);
			props.onClose();
		} catch (e) {
			errorToast("Failed to add package: " + e);
		}
	};

	return (
		<Modal
			visible={props.visible}
			onClose={props.onClose}
			width="60rem"
			height="30rem"
			title={
				<>
					Installing package
					<div style="color:var(--fg3)">{name()}</div>
				</>
			}
			titleIcon={Download}
			buttons={[
				{
					text: "Cancel",
					icon: Delete,
					onClick: props.onClose,
				},
				{
					text: "Install",
					icon: Download,
					onClick: install,
					color: "var(--package)",
					bgColor: "var(--packagebg)",
				},
			]}
		>
			<div id="package-install-inner">
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
					<div style="width:30%">Where would you like to install?</div>
					<div class="cont" style="width:70%">
						<InlineSelect
							options={installLocationOptions()}
							selected={selectedTab()}
							onChange={(tab) => {
								setSelectedTab(tab!);
								setSelectedInstanceOrTemplate(undefined);
							}}
						/>
					</div>
				</div>
				<Show when={selectedTab() != "base_template" && selectedTab() != "new_instance"}>
					<div class="cont">
						<Icon icon={Box} size="1.2rem" />
					</div>
					<div class="cont" id="package-install-target">
						<div>Select {selectedTab()}</div>
					</div>
					<div></div>
					<div class="cont" style="width:100%">
						<Show when={instancesAndTemplates() != undefined}>
							<InlineSelect
								options={(selectedTab() == "instance"
									? instancesAndTemplates()![0]
									: instancesAndTemplates()![1]
								).map((item) => {
									return {
										value: item.id,
										contents: (
											<div>{item.name == undefined ? item.id : item.name}</div>
										),
										color: `var(--${selectedTab()})`,
										selectedBgColor: `var(--${selectedTab()}bg)`,
									};
								})}
								selected={selectedInstanceOrTemplate()}
								onChange={setSelectedInstanceOrTemplate}
								columns={3}
								connected={false}
							/>
						</Show>
					</div>
				</Show>
				<Show when={selectedTab() == "template"}>
					<div class="cont">
						<Icon icon={Diagram} size="1.2rem" />
					</div>
					<div class="cont fullwidth" id="package-install-template-location">
						<div style="width:40%">
							What children of this template should get this package?
						</div>
						<div class="cont" style="width:60%">
							<InlineSelect
								options={[
									{
										value: "all",
										contents: (
											<div class="cont">
												<Icon icon={Globe} size="1rem" /> All of them
											</div>
										),
										color: "var(--package)",
										selectedBgColor: "var(--packagebg)",
									},
									{
										value: "client",
										contents: (
											<div class="cont">
												<Icon icon={Controller} size="1.2rem" /> Clients
											</div>
										),
										color: "var(--package)",
										selectedBgColor: "var(--packagebg)",
									},
									{
										value: "server",
										contents: (
											<div class="cont">
												<Icon icon={Server} size="1rem" /> Servers
											</div>
										),
										color: "var(--package)",
										selectedBgColor: "var(--packagebg)",
									},
								]}
								selected={selectedTemplateLocation()}
								onChange={setSelectedTemplateLocation}
							/>
						</div>
					</div>
				</Show>
				<Show when={selectedTab() == "new_instance"}>
					<div class="cont">
						<Icon icon={Plus} size="1.2rem" />
					</div>
					<div class="cont start fullwidth">
						<div style="width:40%">
							What should be the ID for the new instance?
						</div>
						<div class="cont" style="width:60%" id="package-install-instance-id">
							<input
								id="new-instance-id"
								style="width:100%"
								value={emptyUndefined(newInstanceId())}
								onchange={(e) => setNewInstanceId(e.target.value)}
							/>
						</div>
					</div>
				</Show>
			</div>
		</Modal>
	);
}

export interface PackageInstallModalProps {
	packageId: string;
	packageRepo?: string;
	packageSlug?: string;
	packageName?: string;
	modpackMinecraftVersions?: string[];
	selectedVersion?: string;
	visible: boolean;
	onClose: () => void;
	// Function to show the versions tab
	onShowVersions: () => void;
}
