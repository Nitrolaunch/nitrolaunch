import { createResource, createSignal, Show } from "solid-js";
import "./PackageFilters.css";
import Icon from "../Icon";
import {
	Box,
	Hashtag,
	Jigsaw,
	Lock,
	Minecraft,
	Plus,
	Properties,
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
} from "../../package";
import InlineSelect from "../input/InlineSelect";
import { invoke } from "@tauri-apps/api";
import { PackageType } from "../../package";

export default function PackageFilters(props: PackageFiltersProps) {
	let [tab, setTab] = createSignal(
		props.filteringVersions ? "minecraft_versions" : "types"
	);

	let [versionFilterOptions, _] = createResource(async () => {
		// If a list of versions is available (we are filtering a list of package versions), use taht
		if (props.availableMinecraftVersions != undefined) {
			let versions = props.availableMinecraftVersions.concat([]);
			versions.reverse();
			return versions.slice(0, 7);
		}

		// Let the user select from the most recent couple versions, along with some popular ones
		let availableVersions = (await invoke("get_minecraft_versions", {
			releasesOnly: true,
		})) as string[];

		let latestReleases = availableVersions.slice(
			availableVersions.length - 4,
			availableVersions.length - 1
		);
		latestReleases.reverse();
		let popularVersions = ["1.19.4", "1.18.2", "1.16.5", "1.12.2"];

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
						class={`cont package-filter-tab ${
							tab() == "types" ? "selected" : ""
						}`}
						onclick={() => setTab("types")}
					>
						<Icon icon={Jigsaw} size="0.8rem" />
						Type
					</div>
				</Show>
				<div
					class={`cont package-filter-tab ${
						tab() == "minecraft_versions" ? "selected" : ""
					}`}
					onclick={() => setTab("minecraft_versions")}
					style="color:var(--instance)"
				>
					<Icon icon={Minecraft} size="0.8rem" />
					Version
				</div>
				<div
					class={`cont package-filter-tab ${
						tab() == "loaders" ? "selected" : ""
					}`}
					onclick={() => setTab("loaders")}
					style="color:var(--package)"
				>
					<Icon icon={Box} size="0.8rem" />
					Loader
				</div>
				<Show when={!props.filteringVersions}>
					<div
						class={`cont package-filter-tab ${
							tab() == "categories" ? "selected" : ""
						}`}
						onclick={() => setTab("categories")}
						style="color:var(--profile)"
					>
						<Icon icon={Hashtag} size="0.8rem" />
						Category
					</div>
				</Show>
				<Show when={props.filteringVersions}>
					<div
						class={`cont package-filter-tab ${
							tab() == "stability" ? "selected" : ""
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
						class={`cont package-filter-tab ${
							tab() == "features" ? "selected" : ""
						}`}
						onclick={() => setTab("features")}
						style="color:var(--pluginfg)"
					>
						<Icon icon={Properties} size="0.8rem" />
						Features
					</div>
				</Show>
				<Show when={!props.filteringVersions}>
					<div
						class={`cont package-filter-tab ${
							tab() == "more" ? "selected" : ""
						}`}
						onclick={() => setTab("more")}
						style="color:var(--fg3)"
					>
						<Icon icon={Plus} size="0.8rem" />
						More
					</div>
				</Show>
				<Show when={props.filteringVersions}>
					<div style="height:100%;width:20%;box-sizing:border-box;border-bottom:0.15rem solid var(--bg3)"></div>
				</Show>
			</div>
			<div class="cont package-filter-contents">
				<Show when={tab() == "types"}>
					<div class="cont package-filter-tab-contents" style="padding:0.5rem">
						<InlineSelect
							options={availablePackageTypes().map((packageType) => {
								return {
									value: packageType,
									contents: (
										<div class="cont">
											<Icon
												icon={getPackageTypeIcon(packageType)}
												size="1.2rem"
											/>
											<div style="font-size:0.9rem;font-weight:bold">{`${getPackageTypeDisplayName(
												packageType
											)}s`}</div>
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
							columns={4}
							connected={false}
						/>
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
			</div>
		</div>
	);
}

export interface PackageFiltersProps {
	packageType: PackageType;
	minecraftVersions: string[];
	loaders: string[];
	stability?: "stable" | "latest";
	setPackageType: (type: PackageType) => void;
	setMinecraftVersions: (versions: string[]) => void;
	setLoaders: (loaders: string[]) => void;
	setStability: (stability?: "stable" | "latest") => void;
	availablePackageTypes?: PackageType[];
	availableMinecraftVersions?: string[];
	// Whether we are filtering package versions or packages
	filteringVersions: boolean;
}

function MinecraftVersionsTab(props: MinecraftVersionsTabProps) {
	return (
		<div class="cont package-filter-tab-contents" style="padding:0.5rem">
			<InlineSelect
				options={props.options.map((version) => {
					return {
						value: version,
						contents: (
							<div class="cont">
								<div style="font-size:0.9rem;font-weight:bold;text-align:center">
									{version}
								</div>
							</div>
						),
						color: "var(--instance)",
					};
				})}
				selected={props.selectedVersions}
				onChangeMulti={(values) =>
					props.setMinecraftVersions(values == undefined ? [] : values)
				}
				columns={7}
				connected={false}
			/>
		</div>
	);
}

interface MinecraftVersionsTabProps {
	options: string[];
	selectedVersions: string[];
	setMinecraftVersions: (versions: string[]) => void;
}

// The actual filters that are applied
export interface PackageFilterOptions {
	minecraft_versions: string[];
	loaders: string[];
	stability?: "stable" | "latest";
}

export function defaultPackageFilters() {
	let out: PackageFilterOptions = {
		minecraft_versions: [],
		loaders: [],
	};
	return out;
}
