import "./InstanceList.css";
import {
	createEffect,
	createSignal,
	For,
	Match,
	onMount,
	Show,
	Switch,
} from "solid-js";
import { loadPagePlugins } from "../../plugins";
import { FooterData } from "../../App";
import { FooterMode } from "../../components/navigation/Footer";
import { getInstanceIconSrc } from "../../utils";
import { GroupInfo, InstanceInfo, InstanceMap } from "../../types";
import { errorToast } from "../../components/dialog/Toasts";
import { invoke } from "@tauri-apps/api";
import IconButton from "../../components/input/IconButton";
import {
	Box,
	Download,
	Edit,
	Folder,
	Pin,
	Plus,
	Properties,
} from "../../icons";
import Icon from "../../components/Icon";
import IconTextButton from "../../components/input/IconTextButton";
import InstanceTransferPrompt from "../../components/instance/InstanceTransferPrompt";
import Dropdown from "../../components/input/Dropdown";
import IconAndText from "../../components/utility/IconAndText";
import { useNavigate } from "@solidjs/router";

export default function InstanceList(props: InstanceListProps) {
	let navigate = useNavigate();

	onMount(() => loadPagePlugins("instances"));

	const [instances, setInstances] = createSignal<InstanceInfo[]>([]);
	const [profiles, setProfiles] = createSignal<InstanceInfo[]>([]);
	const [pinned, setPinned] = createSignal<InstanceInfo[]>([]);
	const [groups, setGroups] = createSignal<GroupSectionData[]>([]);
	const [selectedItem, setSelectedItem] = createSignal<
		SelectedItem | undefined
	>(undefined);
	const [selectedSection, setSelectedSection] = createSignal<string | null>(
		null
	);
	const [instancesOrProfiles, setInstancesOrProfiles] = createSignal<
		"instance" | "profile"
	>("instance");

	let [importPromptVisible, setImportPromptVisible] = createSignal(false);

	async function updateItems() {
		let instances: InstanceInfo[] = [];
		let profiles: InstanceInfo[] = [];
		let groups: GroupInfo[] = [];
		try {
			[instances, profiles, groups] = (await Promise.all([
				invoke("get_instances"),
				invoke("get_profiles"),
				invoke("get_instance_groups"),
			])) as [InstanceInfo[], InstanceInfo[], GroupInfo[]];
		} catch (e) {
			errorToast("Failed to get instances: " + e);
		}

		// Create map of instances and put pinned instances in their section
		let newPinned = [];
		let instanceMap: InstanceMap = {};
		for (let instance of instances) {
			if (instance.pinned) {
				newPinned.push(instance);
			}
			instanceMap[instance.id] = instance;
		}
		setPinned(newPinned);
		setInstances(instances);

		let profileMap: InstanceMap = {};
		for (let profile of profiles) {
			profileMap[profile.id] = profile;
		}
		setProfiles(profiles);

		// Create groups
		let newGroups: GroupSectionData[] = [];
		for (let group of groups) {
			let newInstances = [];
			for (let instanceId of group.contents) {
				try {
					let instance = instanceMap[instanceId];
					newInstances.push(instance);
				} catch (e) {
					console.error(
						"Failed to fetch instance '" + instanceId + "' from map"
					);
				}
			}
			const newGroup: GroupSectionData = {
				id: group.id,
				instances: newInstances,
			};
			newGroups.push(newGroup);
		}
		setGroups(newGroups);
	}

	updateItems();

	function onSelect(item: SelectedItem, section: string) {
		setSelectedItem(item);
		setSelectedSection(section);
		props.setFooterData({
			selectedItem: item.id,
			mode: item.type as FooterMode,
			action: () => {},
			fromPlugin: item.fromPlugin,
		});
	}

	createEffect(() => {
		props.setFooterData({
			mode: FooterMode.Instance,
			selectedItem: undefined,
			action: () => {},
		});
	});
	return (
		<div class="container">
			<br />
			<div id="instance-list">
				<div class="cont">
					<div class="split3" style="width: 80%">
						<div class="cont start" style="padding-left:0.5rem">
							<div style="width:30%">
								<Dropdown
									options={[
										{
											value: "create_instance",
											contents: (
												<IconAndText icon={Box} text="Create Instance" />
											),
										},
										{
											value: "create_profile",
											contents: (
												<IconAndText icon={Properties} text="Create Profile" />
											),
										},
										{
											value: "import_instance",
											contents: (
												<IconAndText icon={Download} text="Import Instance" />
											),
										},
									]}
									previewText={<IconAndText icon={Plus} text="Add" />}
									onChange={(selection) => {
										if (selection == "create_instance") {
											navigate("create_instance");
										} else if (selection == "create_profile") {
											navigate("create_profile");
										} else if (selection == "import_instance") {
											setImportPromptVisible(true);
										}
									}}
									optionsWidth="12rem"
									isSearchable={false}
									showArrow={false}
									zIndex="2"
								/>
							</div>
						</div>
						<div id="instance-list-header">
							<div
								class={`instance-list-header-item instances${
									instancesOrProfiles() == "instance" ? " selected" : ""
								}`}
								onclick={() => {
									setInstancesOrProfiles("instance");
								}}
							>
								Instances
							</div>
							<div
								class={`instance-list-header-item profiles${
									instancesOrProfiles() == "profile" ? " selected" : ""
								}`}
								onclick={() => {
									setInstancesOrProfiles("profile");
								}}
							>
								Profiles
							</div>
						</div>
						<div></div>
					</div>
				</div>
				<br />
				<Switch>
					<Match when={instancesOrProfiles() == "instance"}>
						<Show when={pinned().length > 0}>
							<Section
								id="pinned"
								kind="pinned"
								header="PINNED"
								items={pinned()}
								selectedItem={selectedItem()}
								selectedSection={selectedSection()}
								onSelectItem={onSelect}
								updateList={updateItems}
								itemType="instance"
							/>
						</Show>
						<For each={groups()}>
							{(item) => (
								<Section
									id={`group-${item.id}`}
									kind="group"
									header={item.id.toLocaleUpperCase()}
									items={item.instances}
									selectedItem={selectedItem()}
									selectedSection={selectedSection()}
									onSelectItem={onSelect}
									updateList={updateItems}
									itemType="instance"
								/>
							)}
						</For>
						<Section
							id="all"
							kind="all"
							header="ALL INSTANCES"
							items={instances()}
							selectedItem={selectedItem()}
							selectedSection={selectedSection()}
							onSelectItem={onSelect}
							updateList={updateItems}
							itemType="instance"
						/>
					</Match>
					<Match when={instancesOrProfiles() == "profile"}>
						<br />
						<div class="cont">
							<IconTextButton
								icon={Edit}
								text="Edit Global Profile"
								size="20px"
								color="var(--bg2)"
								selectedColor="var(--instance)"
								onClick={() => {
									navigate("/global_profile_config");
								}}
								selected={false}
							/>
						</div>
						<br />
						<Section
							id="profiles"
							kind="profiles"
							header="ALL PROFILES"
							items={profiles()}
							selectedItem={selectedItem()}
							selectedSection={selectedSection()}
							onSelectItem={onSelect}
							updateList={updateItems}
							itemType="profile"
						/>
					</Match>
				</Switch>
			</div>
			<InstanceTransferPrompt
				exportedInstance={undefined}
				visible={importPromptVisible()}
				onClose={() => setImportPromptVisible(false)}
			/>
			<br />
		</div>
	);
}

// A section of items, like pinned or an Nitrolaunch instance group
function Section(props: SectionProps) {
	let navigate = useNavigate();

	const HeaderIcon = () => (
		<Switch>
			<Match when={props.kind == "all" || props.kind == "profiles"}>
				<Icon icon={Box} size="18px" />
			</Match>
			<Match when={props.kind == "pinned"}>
				<Icon icon={Pin} size="18px" />
			</Match>
			<Match when={props.kind == "group"}>
				<Icon icon={Folder} size="18px" />
			</Match>
		</Switch>
	);

	return (
		<div class="cont col">
			<div class="cont col instance-list-section-container">
				<div class="cont instance-list-section-header">
					<HeaderIcon />
					<h2>{props.header}</h2>
				</div>
				<div class="instance-list-section">
					<For each={props.items}>
						{(item) => (
							<Item
								instance={item}
								selected={
									props.selectedSection !== null &&
									props.selectedSection === props.id &&
									props.selectedItem?.id === item.id
								}
								onSelect={() => {
									props.onSelectItem(
										{
											id: item.id,
											type: props.itemType,
											fromPlugin: item.from_plugin,
										},
										props.id
									);
								}}
								sectionKind={props.kind}
								itemKind={props.itemType}
								updateList={props.updateList}
							/>
						)}
					</For>
					{/* Button for creating a new instance */}
					<Show when={props.kind == "all" || props.kind == "profiles"}>
						<div
							class="input-shadow instance-list-item noselect"
							onclick={() => {
								let target =
									props.itemType == "instance"
										? "create_instance"
										: "create_profile";
								navigate(target);
							}}
						>
							<div class="cont instance-list-icon">
								<Icon icon={Plus} size="1.5rem" />
							</div>
							<div style="" class="bold">
								{`Create ${
									props.itemType == "instance" ? "Instance" : "Profile"
								}`}
							</div>
						</div>
					</Show>
				</div>
			</div>
		</div>
	);
}

interface SectionProps {
	id: string;
	kind: SectionKind;
	itemType: "instance" | "profile";
	header: string;
	items: InstanceInfo[];
	selectedItem?: SelectedItem;
	selectedSection: string | null;
	onSelectItem: (item: SelectedItem, section: string) => void;
	updateList: () => void;
}

type SectionKind = "pinned" | "group" | "all" | "profiles";

interface GroupSectionData {
	id: string;
	instances: InstanceInfo[];
}

function Item(props: ItemProps) {
	let navigate = useNavigate();

	const [isHovered, setIsHovered] = createSignal(false);

	let icon =
		props.instance.icon == undefined ? (
			<div class="cont instance-list-icon">
				<Icon icon={Box} size="2.1rem" />
			</div>
		) : (
			<img
				src={getInstanceIconSrc(props.instance.icon)}
				class="instance-list-icon"
			/>
		);

	return (
		<div
			class={`input-shadow instance-list-item noselect ${
				props.selected ? "selected" : ""
			} ${props.itemKind}`}
			onClick={() => {
				// Double click to edit
				if (props.selected) {
					let url =
						props.itemKind == "instance"
							? `/instance/${props.instance.id}`
							: `/profile_config/${props.instance.id}`;
					navigate(url);
				} else {
					props.onSelect();
				}
			}}
			onMouseEnter={() => setIsHovered(true)}
			onMouseLeave={() => setIsHovered(false)}
		>
			{/* Don't show the pin button when the instance is already pinned and we aren't in the pinned section */}
			<Show
				when={
					isHovered() &&
					props.itemKind == "instance" &&
					!(props.instance.pinned && props.sectionKind !== "pinned")
				}
			>
				<div class="instance-list-pin">
					<IconButton
						icon={Pin}
						size="1.7rem"
						color="transparent"
						selectedColor="var(--instance)"
						iconColor={
							props.sectionKind == "pinned" ? "var(--bg2)" : "var(--fg)"
						}
						onClick={(e) => {
							// Don't select the instance
							e.stopPropagation();
							invoke("pin_instance", {
								instanceId: props.instance.id,
								pin: !props.instance.pinned,
							}).then(props.updateList, (e) => {
								errorToast("Failed to pin instance: " + e);
							});
						}}
						selected={props.sectionKind === "pinned"}
						hoverBackground="var(--bg4)"
						circle
					/>
				</div>
			</Show>
			{icon}
			<div class="instance-list-item-details">
				<div style="" class="bold">
					{props.instance.name !== null
						? props.instance.name
						: props.instance.id}
				</div>
				<Show when={props.instance.name !== null}>
					<div style="color: var(--fg3)">{props.instance.id}</div>
				</Show>
			</div>
		</div>
	);
}

interface ItemProps {
	instance: InstanceInfo;
	selected: boolean;
	sectionKind: SectionKind;
	itemKind: "instance" | "profile";
	onSelect: () => void;
	updateList: () => void;
}

interface SelectedItem {
	id?: string;
	type: "instance" | "profile";
	fromPlugin: boolean;
}

export interface InstanceListProps {
	setFooterData: (data: FooterData) => void;
}
