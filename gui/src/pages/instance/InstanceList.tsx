import "./InstanceList.css";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	Match,
	onMount,
	Show,
	Switch,
} from "solid-js";
import { dropdownButtonToOption, getDropdownButtons, loadPagePlugins, runDropdownButtonClick } from "../../plugins";
import { FooterData } from "../../App";
import { FooterMode } from "../../components/navigation/Footer";
import { getInstanceIconSrc } from "../../utils";
import { GroupInfo, InstanceInfo, InstanceMap, InstanceOrTemplate } from "../../types";
import { errorToast } from "../../components/dialog/Toasts";
import { invoke } from "@tauri-apps/api/core";
import IconButton from "../../components/input/button/IconButton";
import {
	Box,
	Controller,
	Cycle,
	Diagram,
	Download,
	Folder,
	Globe,
	Honeycomb,
	Info,
	Jigsaw,
	Pin,
	Plus,
	Properties,
	Server,
	Tag,
} from "../../icons";
import Icon from "../../components/Icon";
import IconTextButton from "../../components/input/button/IconTextButton";
import InstanceTransferPrompt from "../../components/instance/InstanceTransferPrompt";
import Dropdown, { Option } from "../../components/input/select/Dropdown";
import IconAndText from "../../components/utility/IconAndText";
import { useNavigate } from "@solidjs/router";
import MigratePrompt from "../../components/instance/MigratePrompt";
import Tip from "../../components/dialog/Tip";

export default function InstanceList(props: InstanceListProps) {
	let navigate = useNavigate();

	onMount(() => loadPagePlugins("instances"));

	const [instances, setInstances] = createSignal<InstanceInfo[]>([]);
	const [templates, setTemplates] = createSignal<InstanceInfo[]>([]);
	const [pinned, setPinned] = createSignal<InstanceInfo[]>([]);
	const [groups, setGroups] = createSignal<GroupSectionData[]>([]);
	const [selectedItem, setSelectedItem] = createSignal<
		SelectedItem | undefined
	>(undefined);
	const [selectedSection, setSelectedSection] = createSignal<string | null>(
		null
	);
	const [instancesOrTemplates, setInstancesOrTemplates] = createSignal<
		"instance" | "template"
	>("instance");

	let [importPromptVisible, setImportPromptVisible] = createSignal(false);
	let [migratePromptVisible, setMigratePromptVisible] = createSignal(false);

	async function updateItems() {
		let instances: InstanceInfo[] = [];
		let templates: InstanceInfo[] = [];
		let groups: GroupInfo[] = [];
		try {
			[instances, templates, groups] = (await Promise.all([
				invoke("get_instances"),
				invoke("get_templates"),
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

		let templateMap: InstanceMap = {};
		for (let template of templates) {
			templateMap[template.id] = template;
		}
		setTemplates(templates);

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

	(window as any).__updateInstanceList = updateItems;

	updateItems();

	function onSelect(item: SelectedItem, section: string) {
		setSelectedItem(item);
		setSelectedSection(section);
		props.setFooterData({
			selectedItem: item.id,
			mode: item.type as FooterMode,
			action: () => { },
			fromPlugin: item.fromPlugin,
		});
	}

	createEffect(() => {
		props.setFooterData({
			mode: FooterMode.Instance,
			selectedItem: undefined,
			action: () => { },
		});
	});

	let [dropdownButtons, _] = createResource(async () => {
		return getDropdownButtons("add_template_or_instance")
	}, { initialValue: [] });

	return (
		<div class="cont col">
			<br />
			<div id="instance-list">
				<div class="cont">
					<div class="fullwidth" id="instance-list-top">
						<div class="cont start" style="padding-left:0.5rem">
							<div style="width:7rem">
								<Dropdown
									options={([
										{
											value: "create_instance",
											contents: (
												<IconAndText icon={Box} text="Create Instance" />
											),
										},
										{
											value: "create_template",
											contents: (
												<IconAndText icon={Properties} text="Create Template" />
											),
										},
										{
											value: "import_instance",
											contents: (
												<IconAndText icon={Download} text="Import Instance" />
											),
										},
										{
											value: "migrate_instances",
											contents: (
												<IconAndText icon={Cycle} text="Migrate Instances" />
											),
										},
									] as Option[]).concat(dropdownButtons().map(dropdownButtonToOption))}
									previewText={<IconAndText icon={Plus} text="Add" centered />}
									onChange={(selection) => {
										if (selection == "create_instance") {
											navigate("create_instance");
										} else if (selection == "create_template") {
											navigate("create_template");
										} else if (selection == "import_instance") {
											setImportPromptVisible(true);
										} else if (selection == "migrate_instances") {
											setMigratePromptVisible(true);
										} else {
											runDropdownButtonClick(selection!);
										}
									}}
									optionsWidth="13rem"
									isSearchable={false}
									showArrow={false}
									zIndex="2"
								/>
							</div>
						</div>
						<div class="cont">
							<div
								class={`cont instance-list-header-item bubble-hover instances ${instancesOrTemplates() == "instance" ? "selected" : ""
									}`}
								onclick={() => {
									setInstancesOrTemplates("instance");
								}}
							>
								<Icon icon={Honeycomb} size="1rem" />
								Instances
							</div>
						</div>
						<div class="cont end" style="padding-right:0.5rem">
							<div
								class={`cont instance-list-header-item bubble-hover templates ${instancesOrTemplates() == "template" ? "selected" : ""
									}`}
								style="width:18rem"
								onclick={() => {
									setInstancesOrTemplates("template");
								}}
							>
								<Icon icon={Diagram} size="1rem" />
								Instance Templates
							</div>
						</div>
					</div>
				</div>
				<br />
				<Switch>
					<Match when={instancesOrTemplates() == "instance"}>
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
					<Match when={instancesOrTemplates() == "template"}>
						<div class="cont fullwidth" id="instance-list-templates-header">
							<div class="cont start fullwidth">
								<Tip tip="Edit the template that all instances and templates inherit from" fullwidth side="top">
									<IconTextButton
										icon={Globe}
										text="Edit Base Template"
										size="1.5rem"
										onClick={() => {
											navigate("/base_template_config");
										}}
									/>
								</Tip>
							</div>
							<div class="cont end bold fullwidth" style="color:var(--fg3);text-align:right;text-wrap:nowrap">
								<Icon icon={Info} size="1rem" />
								Templates let you share settings between multiple instances
							</div>
						</div>
						<div></div>
						<Section
							id="templates"
							kind="templates"
							header="ALL TEMPLATES"
							items={templates()}
							selectedItem={selectedItem()}
							selectedSection={selectedSection()}
							onSelectItem={onSelect}
							updateList={updateItems}
							itemType="template"
						/>
					</Match>
				</Switch>
			</div>
			<InstanceTransferPrompt
				exportedInstance={undefined}
				visible={importPromptVisible()}
				onClose={() => setImportPromptVisible(false)}
			/>
			<MigratePrompt visible={migratePromptVisible()} onClose={() => setMigratePromptVisible(false)} />
			<br />
		</div>
	);
}

// A section of items, like pinned or an Nitrolaunch instance group
function Section(props: SectionProps) {
	let navigate = useNavigate();

	const HeaderIcon = () => (
		<Switch>
			<Match when={props.kind == "all" || props.kind == "templates"}>
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
				<Show when={props.itemType == "instance"}>
					<div class="cont instance-list-section-header">
						<HeaderIcon />
						<h2>{props.header}</h2>
					</div>
				</Show>
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
					<Show when={props.kind == "all" || props.kind == "templates"}>
						<div
							class="shadow instance-list-item bubble-hover-small noselect"
							onclick={() => {
								let target =
									props.itemType == "instance"
										? "create_instance"
										: "create_template";
								navigate(target);
							}}
						>
							<div class="cont instance-list-icon">
								<Icon icon={Plus} size="1.5rem" />
							</div>
							<div style="" class="bold">
								{`Create ${props.itemType == "instance" ? "Instance" : "Template"
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
	itemType: InstanceOrTemplate;
	header: string;
	items: InstanceInfo[];
	selectedItem?: SelectedItem;
	selectedSection: string | null;
	onSelectItem: (item: SelectedItem, section: string) => void;
	updateList: () => void;
}

type SectionKind = "pinned" | "group" | "all" | "templates";

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
			class={`shadow bubble-hover-small instance-list-item noselect ${props.selected ? "selected" : ""
				} ${props.itemKind}`}
			onClick={() => {
				// Double click to edit
				if (props.selected) {
					if (!props.instance.from_plugin) {
						let url =
							props.itemKind == "instance"
								? `/instance/${props.instance.id}`
								: `/template_config/${props.instance.id}`;
						navigate(url);
					}
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
			<div class="cont col instance-list-item-details">
				<div class="cont start" style="text-wrap:nowrap">
					<span class="bold">
						{props.instance.name !== null
							? props.instance.name
							: props.instance.id}
					</span>

					<Show when={props.instance.from_plugin}>
						<div class="cont" style="color:var(--fg)">
							<Tip tip="Created by plugin" side="top">
								<div class="cont" style="color:var(--plugin)">
									<Icon icon={Jigsaw} size="1rem" />
								</div>
							</Tip>
						</div>
					</Show>
					<Show when={props.instance.name !== null}>
						<div class="cont start" style="color:var(--fg3)">{props.instance.id}</div>
					</Show>
				</div>
				<div class="cont start bold" style="color: var(--fg3);gap:0.7rem;font-size:0.9rem;margin-left:-0.1rem">
					<Show when={props.instance.side != undefined}>
						<div class="cont" style="gap:0.3rem">
							<Switch>
								<Match when={props.instance.side == "client"}>
									<Icon icon={Controller} size="1.2rem" />
									<span style="transform:translateY(0.1em)">
										Client
									</span>
								</Match>
								<Match when={props.instance.side == "server"}>
									<Icon icon={Server} size="1rem" />
									<span style="transform:translateY(0.1em)">
										Server
									</span>
								</Match>
							</Switch>
						</div>
					</Show>
					<Show when={props.instance.version != undefined}>
						<div class="cont" style="gap:0.3rem">
							<Icon icon={Tag} size="0.8rem" />
							<span style="transform:translateY(0.1em)">
								{props.instance.version}
							</span>
						</div>
					</Show>
				</div>
			</div>
		</div>
	);
}

interface ItemProps {
	instance: InstanceInfo;
	selected: boolean;
	sectionKind: SectionKind;
	itemKind: InstanceOrTemplate;
	onSelect: () => void;
	updateList: () => void;
}

interface SelectedItem {
	id?: string;
	type: InstanceOrTemplate;
	fromPlugin: boolean;
}

export interface InstanceListProps {
	setFooterData: (data: FooterData) => void;
}

export async function updateInstanceList() {
	await (window as any).__updateInstanceList();
}
