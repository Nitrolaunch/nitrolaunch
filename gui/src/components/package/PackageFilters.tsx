import { createResource, createSignal, Show } from "solid-js";
import "./PackageFilters.css";
import Icon from "../Icon";
import {
	Box,
	Jigsaw,
	Lock,
	Minecraft,
	Plus,
	Properties,
	Tag,
	Trash,
	Warning,
} from "../../icons";
import {
	getLoaderColor,
	getLoaderDisplayName,
	getLoaderImage,
	getPackageTypeColor,
	getPackageTypeDisplayName,
	getPackageTypeIcon,
	Loader,
	PackageCategory,
	packageCategoryDisplayName,
	packageCategoryIcon,
} from "../../package";
import InlineSelect from "../input/select/InlineSelect";
import { invoke } from "@tauri-apps/api";
import { PackageType } from "../../package";
import Dropdown from "../input/select/Dropdown";
import { beautifyString, fixCenter } from "../../utils";
import IconTextButton from "../input/button/IconTextButton";
import IconAndText from "../utility/IconAndText";

export default function PackageFilters(props: PackageFiltersProps) {
	let [tab, setTab] = createSignal(
		props.filteringVersions ? "minecraft_versions" : "types"
	);

	let [extraMinecraftVersions, setExtraMinecraftVersions] = createSignal<
		string[]
	>([]);

	let [versionFilterOptions, _] = createResource(async () => {
		// If a list of versions is available (we are filtering a list of package versions), use taht
		if (props.availableMinecraftVersions != undefined) {
			let versions = props.availableMinecraftVersions.concat([]);
			versions.reverse();
			setExtraMinecraftVersions(versions.slice(5));
			return versions.slice(0, 4);
		}

		// Let the user select from the most recent couple versions, along with some popular ones
		let availableVersions = (await invoke("get_minecraft_versions", {
			releasesOnly: true,
		})) as string[];

		availableVersions.reverse();
		let latestReleases = availableVersions.slice(0, 1);
		let popularVersions = ["1.19.4", "1.18.2", "1.16.5", "1.12.2"];

		setExtraMinecraftVersions(availableVersions.slice(1));

		return latestReleases.concat(popularVersions);
	});

	let availablePackageTypes = () =>
		props.availablePackageTypes == undefined
			? ([
				"mod",
				"resource_pack",
				"datapack",
				"plugin",
				"shader",
				"bundle",
			] as PackageType[])
			: props.availablePackageTypes;

	return (
		<div class="package-filters">
			<div class="cont package-filters-tabs">
				<Show when={!props.filteringVersions}>
					<div
						class={`cont package-filter-tab ${tab() == "types" ? "selected" : ""
							}`}
						onclick={() => setTab("types")}
					>
						<Icon icon={Jigsaw} size="0.8rem" />
						Type
					</div>
				</Show>
				<div
					class={`cont package-filter-tab ${tab() == "minecraft_versions" ? "selected" : ""
						}`}
					onclick={() => setTab("minecraft_versions")}
					style="color:var(--instance)"
				>
					<Icon icon={Minecraft} size="0.8rem" />
					Version
				</div>
				<div
					class={`cont package-filter-tab ${tab() == "loaders" ? "selected" : ""
						}`}
					onclick={() => setTab("loaders")}
					style="color:var(--package)"
				>
					<Icon icon={Box} size="0.8rem" />
					Loader
				</div>
				<Show when={!props.filteringVersions}>
					<div
						class={`cont package-filter-tab ${tab() == "categories" ? "selected" : ""
							}`}
						onclick={() => setTab("categories")}
						style="color:var(--profile)"
					>
						<Icon icon={Tag} size="0.8rem" />
						Category
					</div>
				</Show>
				<Show when={props.filteringVersions}>
					<div
						class={`cont package-filter-tab ${tab() == "stability" ? "selected" : ""
							}`}
						onclick={() => setTab("stability")}
						style="color:var(--profile)"
					>
						<Icon icon={Lock} size="0.8rem" />
						Stability
					</div>
				</Show>
				<Show when={props.filteringVersions}>
					<div
						class={`cont package-filter-tab ${tab() == "features" ? "selected" : ""
							}`}
						onclick={() => setTab("features")}
						style="color:var(--pluginfg)"
					>
						<Icon icon={Properties} size="0.8rem" />
						Features
					</div>
				</Show>
				<div
					class={`cont package-filter-tab ${tab() == "more" ? "selected" : ""}`}
					onclick={() => setTab("more")}
					style="color:var(--fg3)"
				>
					<Icon icon={Plus} size="0.8rem" />
					More
				</div>
			</div>
			<div class="cont package-filter-contents">
				<Show when={tab() == "types"}>
					<div class="cont package-filter-tab-contents" style="padding:0.5rem">
						<InlineSelect
							options={availablePackageTypes().map((packageType) => {
								return {
									value: packageType,
									contents: (
										<div class="cont" style="font-size:0.9rem;font-weight:bold">
											<Icon
												icon={getPackageTypeIcon(packageType)}
												size="1.2rem"
											/>
											<div class="cont" style={fixCenter(getPackageTypeDisplayName(packageType))}>
												{`${getPackageTypeDisplayName(
													packageType
												)}s`}
											</div>
										</div>
									),
									color: getPackageTypeColor(packageType),
									tip: packageType == "bundle" ? "AKA Modpacks" : undefined,
								};
							})}
							selected={props.packageType}
							onChange={(value) => props.setPackageType(value as PackageType)}
							columns={availablePackageTypes().length}
							connected={false}
						/>
					</div>
				</Show>
				<Show
					when={
						tab() == "minecraft_versions" && versionFilterOptions() != undefined
					}
				>
					<MinecraftVersionsTab
						options={versionFilterOptions()!}
						extraOptions={extraMinecraftVersions()}
						selectedVersions={props.minecraftVersions}
						setMinecraftVersions={props.setMinecraftVersions}
					/>
				</Show>
				<Show when={tab() == "loaders"}>
					<div class="cont package-filter-tab-contents" style="padding:0.5rem">
						<InlineSelect
							options={[
								Loader.Fabric,
								Loader.Forge,
								Loader.NeoForge,
								Loader.Quilt,
								Loader.Paper,
								Loader.Folia,
								Loader.Sponge,
								Loader.SpongeForge,
							].map((loader) => {
								return {
									value: loader,
									contents: (
										<div class="cont">
											<img src={getLoaderImage(loader)} style="width:1.2rem" />
											<div style="font-size:0.9rem;font-weight:bold">
												{getLoaderDisplayName(loader)}
											</div>
										</div>
									),
									color: getLoaderColor(loader),
								};
							})}
							selected={props.loaders}
							onChangeMulti={(values) =>
								props.setLoaders(values == undefined ? [] : values)
							}
							columns={props.filteringVersions ? 4 : 6}
							connected={false}
						/>
					</div>
				</Show>
				<Show when={tab() == "categories"}>
					<div class="cont package-filter-tab-contents" style="padding:0.5rem">
						<div class="cont" style="width:calc(400%/5)">
							<InlineSelect
								options={[
									PackageCategory.Adventure,
									PackageCategory.Building,
									PackageCategory.Optimization,
									PackageCategory.Magic,
								]
									.filter(
										(x) =>
											props.availableCategories == undefined ||
											props.availableCategories.includes(x)
									)
									.map((category) => {
										return {
											value: category,
											contents: (
												<div class="cont">
													<Icon
														icon={packageCategoryIcon(category)}
														size="1rem"
													/>
													<div class="cont">
														{packageCategoryDisplayName(category)}
													</div>
												</div>
											),
											color: "var(--profile)",
										};
									})}
								connected={false}
								columns={4}
								selected={props.categories}
								onChangeMulti={(x) =>
									props.setCategories(x as PackageCategory[])
								}
							/>
						</div>
						<div class="cont" style="width:calc(100%/5)">
							<Dropdown
								options={Object.values(PackageCategory)
									.filter(
										(x) =>
											props.availableCategories == undefined ||
											props.availableCategories.includes(x)
									)
									.map((category) => {
										return {
											value: category,
											contents: (
												<IconAndText
													icon={packageCategoryIcon(category)}
													text={packageCategoryDisplayName(category)}
												/>
											),
											color: "var(--profile)",
										};
									})}
								selected={props.categories}
								onChangeMulti={(x) =>
									props.setCategories(x as PackageCategory[])
								}
								isSearchable={false}
								zIndex="20"
							/>
						</div>
					</div>
				</Show>
				<Show when={tab() == "stability"}>
					<div class="cont package-filter-tab-contents" style="padding:0.5rem">
						<InlineSelect
							options={[
								{
									value: "stable",
									contents: (
										<div class="cont">
											<Icon icon={Lock} size="1.2rem" />
											<div style="font-size:0.9rem;font-weight:bold">
												Stable
											</div>
										</div>
									),
									color: "var(--instance)",
								},
								{
									value: "latest",
									contents: (
										<div class="cont">
											<Icon icon={Warning} size="1.2rem" />
											<div
												class="cont"
												style="font-size:0.9rem;font-weight:bold"
											>
												Development
											</div>
										</div>
									),
									color: "var(--warning)",
								},
							]}
							allowEmpty
							selected={props.stability}
							onChange={(x) =>
								props.setStability(x as "stable" | "latest" | undefined)
							}
							columns={3}
							connected={false}
						/>
					</div>
				</Show>
				<Show
					when={tab() == "features" && props.availableFeatures != undefined}
				>
					<div class="cont package-filter-tab-contents" style="padding:0.5rem">
						<InlineSelect
							options={props.availableFeatures!.map((feature) => {
								return {
									value: feature,
									contents: <div class="cont">{beautifyString(feature)}</div>,
									color: "var(--pluginfg)",
								};
							})}
							selected={props.features}
							onChangeMulti={(values) =>
								props.setFeatures(values == undefined ? [] : values)
							}
							columns={props.availableFeatures!.length}
							connected={false}
						/>
					</div>
				</Show>
				<Show when={tab() == "more"}>
					<div class="cont start fullwidth" style="padding:0.5rem">
						<IconTextButton
							icon={Trash}
							size="1rem"
							text="Clear Filters"
							onClick={() => {
								props.setMinecraftVersions([]);
								props.setLoaders([]);
								props.setFeatures([]);
								props.setCategories([]);
								props.setStability(undefined);
							}}
						/>
					</div>
				</Show>
			</div>
		</div>
	);
}

export interface PackageFiltersProps {
	packageType: PackageType;
	minecraftVersions: string[];
	loaders: string[];
	stability?: "stable" | "latest";
	features: string[];
	categories: PackageCategory[];
	setPackageType: (type: PackageType) => void;
	setMinecraftVersions: (versions: string[]) => void;
	setLoaders: (loaders: string[]) => void;
	setStability: (stability?: "stable" | "latest") => void;
	setFeatures: (features: string[]) => void;
	setCategories: (categories: PackageCategory[]) => void;
	availablePackageTypes?: PackageType[];
	availableMinecraftVersions?: string[];
	availableFeatures?: string[];
	availableCategories?: PackageCategory[];
	// Whether we are filtering package versions or packages
	filteringVersions: boolean;
}

function MinecraftVersionsTab(props: MinecraftVersionsTabProps) {
	return (
		<div class="cont package-filter-tab-contents" style="padding:0.5rem">
			<div
				class="cont"
				style={
					props.options.length > 5
						? "width:calc(100%/7*6)"
						: "width:calc(100%/5*4)"
				}
			>
				<InlineSelect
					options={props.options.map((version) => {
						return {
							value: version,
							contents: (
								<div style="font-size:0.9rem;font-weight:bold;text-align:center;width:100%;overflow-x:auto;text-wrap:nowrap">
									{version}
								</div>
							),
							color: "var(--instance)",
						};
					})}
					selected={props.selectedVersions}
					onChangeMulti={(values) =>
						props.setMinecraftVersions(values == undefined ? [] : values)
					}
					columns={props.options.length}
					connected={false}
				/>
			</div>
			<div
				class="cont"
				style={`${props.options.length > 5
					? "width:calc(100%/7*1)"
					: "width:calc(100%/5*1)"
					};height:100%`}
			>
				<Dropdown
					options={props.extraOptions.map((version) => {
						return {
							value: version,
							contents: <div>{version}</div>,
							color: "var(--instance)",
						};
					})}
					selected={props.selectedVersions}
					onChangeMulti={(versions) => {
						props.setMinecraftVersions(versions as string[]);
					}}
					zIndex="5"
				/>
			</div>
		</div>
	);
}

interface MinecraftVersionsTabProps {
	// The main options visible outside the dropdown
	options: string[];
	// The extra options inside of the dropdown
	extraOptions: string[];
	selectedVersions: string[];
	setMinecraftVersions: (versions: string[]) => void;
}

// The actual filters that are applied
export interface PackageFilterOptions {
	minecraft_versions: string[];
	loaders: string[];
	stability?: "stable" | "latest";
	categories: PackageCategory[];
}

export function defaultPackageFilters() {
	let out: PackageFilterOptions = {
		minecraft_versions: [],
		loaders: [],
		categories: [],
	};
	return out;
}
