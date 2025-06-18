import { createResource, createSignal, For, Show } from "solid-js";
import { PackageProperties } from "../../types";
import "./PackageVersions.css";
import { invoke } from "@tauri-apps/api";
import {
	DeclarativePackage,
	PackageAddon,
	PackageVersion,
} from "../../package";
import PackageLabels, { getAllLoaders } from "./PackageLabels";
import Tip from "../dialog/Tip";
import PackageFilters from "./PackageFilters";
import LoadingSpinner from "../utility/LoadingSpinner";
import SearchBar from "../input/SearchBar";
import { canonicalizeListOrSingle } from "../../utils";
import { errorToast } from "../dialog/Toasts";

export default function PackageVersions(props: PackageVersionsProps) {
	let [isScriptPackage, setIsScriptPackage] = createSignal(false);

	let [search, setSearch] = createSignal("");
	let [filteredMinecraftVersions, setFilteredMinecraftVersions] = createSignal<
		string[]
	>([]);
	let [filteredLoaders, setFilteredLoaders] = createSignal<string[]>([]);
	let [filteredStability, setFilteredStability] = createSignal<
		"stable" | "latest" | undefined
	>(undefined);

	let [versions, _] = createResource(async () => {
		try {
			let declarativeContents: DeclarativePackage | undefined = await invoke(
				"get_declarative_package_contents",
				{ package: props.packageId }
			);

			// If this is a script package, just use the content versions
			if (declarativeContents == undefined) {
				setIsScriptPackage(true);

				if (props.props.content_versions == undefined) {
					return undefined;
				}

				let versions = props.props.content_versions.map((version) => {
					return { name: version } as PackageVersion;
				});
				return versions;
			}

			// Combine the same content version across multiple addons into a single version if possible
			let versionsWithIds: { [id: string]: PackageVersion } = {};
			let versionsWithoutIds: PackageVersion[] = [];

			if (declarativeContents.addons == undefined) {
				return [];
			}

			for (let addonId of Object.keys(declarativeContents.addons)) {
				let addon = declarativeContents.addons[addonId];
				let packageAddon: PackageAddon = { id: addonId, kind: addon.kind };
				if (addon.versions == undefined) {
					continue;
				}

				for (let version of addon.versions) {
					let contentVersion =
						version.content_versions == undefined ||
						version.content_versions.length == 0
							? undefined
							: Array.isArray(version.content_versions)
							? version.content_versions[0]
							: version.content_versions;

					let newVersion: PackageVersion = {
						id: version.version,
						name: contentVersion,
						addons: [packageAddon],
						minecraft_versions: version.minecraft_versions,
						side: version.side,
						loaders: version.loaders,
						stability: version.stability,
						features: version.features,
						operating_systems: version.operating_systems,
						architectures: version.architectures,
						languages: version.languages,
					};

					// Add a new version or append an addon to one that already exists
					if (contentVersion == undefined) {
						versionsWithoutIds.push(newVersion);
					} else {
						if (versionsWithIds[contentVersion] == undefined) {
							versionsWithIds[contentVersion] = newVersion;
						} else {
							versionsWithIds[contentVersion].addons.push(packageAddon);
						}
					}
				}
			}

			return Object.values(versionsWithIds).concat(versionsWithoutIds);
		} catch (e) {
			errorToast("Failed to load versions: " + e);
			return undefined;
		}
	});

	// The list of available Minecraft versions for the filters
	let availableMinecraftVersions = () => {
		if (props.props.supported_versions != undefined) {
			return canonicalizeListOrSingle(props.props.supported_versions);
		}
		if (versions() == undefined) {
			return [];
		}

		let allVersions = new Set<string>();
		for (let version of versions()!) {
			for (let mcVersion of canonicalizeListOrSingle(
				version.minecraft_versions
			)) {
				allVersions.add(mcVersion);
			}
		}

		let out = [];
		for (let version of allVersions) {
			out.push(version);
		}

		return out;
	};

	return (
		<div class="cont col package-versions">
			<Show
				when={versions() != undefined}
				fallback={<LoadingSpinner size="5rem" />}
			>
				<div class="cont package-versions-header">
					<div
						class="cont package-versions-count"
						style="justify-content:flex-start"
					>
						{versions()!.length} versions
					</div>
					<div class="cont" style="justify-content:flex-end">
						<SearchBar
							method={(search) => {
								setSearch(search);
							}}
							immediate
						/>
					</div>
				</div>
				<PackageFilters
					packageType={"mod"}
					minecraftVersions={filteredMinecraftVersions()}
					loaders={filteredLoaders()}
					stability={filteredStability()}
					setPackageType={() => {}}
					setMinecraftVersions={setFilteredMinecraftVersions}
					setLoaders={setFilteredLoaders}
					setStability={setFilteredStability}
					filteringVersions={true}
					availableMinecraftVersions={availableMinecraftVersions()}
				/>
				<For each={versions()}>
					{(version) => {
						let isVisible = () => {
							if (
								search() != undefined &&
								version.name != undefined &&
								!version.name!.includes(search()!)
							) {
								return false;
							}

							if (filteredLoaders().length > 0) {
								let found = false;
								let allLoaders = getAllLoaders(
									canonicalizeListOrSingle(version.loaders)
								);
								for (let loader of allLoaders) {
									if (filteredLoaders().includes(loader)) {
										found = true;
										break;
									}
								}

								if (!found) {
									return false;
								}
							}

							if (filteredMinecraftVersions().length > 0) {
								let found = false;
								for (let mcVersion of canonicalizeListOrSingle(
									version.minecraft_versions
								)) {
									if (filteredMinecraftVersions().includes(mcVersion)) {
										found = true;
										break;
									}
								}

								if (!found) {
									return false;
								}
							}

							if (filteredStability() != undefined) {
								console.log(filteredStability(), version.stability);
								if (version.stability != filteredStability()) {
									return false;
								}
							}

							return true;
						};

						return (
							<Show when={isVisible()}>
								<PackageVersionEntry
									version={version}
									backgroundColor={props.backgroundColor}
								/>
							</Show>
						);
					}}
				</For>
			</Show>
		</div>
	);
}

export interface PackageVersionsProps {
	packageId: string;
	props: PackageProperties;
	backgroundColor: string;
}

function PackageVersionEntry(props: PackageVersionEntryProps) {
	let version = props.version;

	let name =
		version.name == undefined
			? version.id == undefined
				? "Unknown"
				: version.id
			: version.name;

	let minecraftVersions = canonicalizeListOrSingle(version.minecraft_versions);

	if (version.name == "8.0.0-neo") {
		console.log(version);
	}

	// Make the font size smaller if there is a long version
	let smallFontSize = false;
	for (let version of minecraftVersions) {
		if (version.length > 8) {
			smallFontSize = true;
		}
	}

	let versions = (
		<Show when={version.minecraft_versions != undefined}>
			<For each={minecraftVersions}>
				{(version, i) => {
					if (i() > 1) {
						if (i() == 2) {
							return <div>...</div>;
						}
						return undefined;
					} else {
						return <div>{version}</div>;
					}
				}}
			</For>
		</Show>
	);

	let loaders = canonicalizeListOrSingle(version.loaders);

	return (
		<div
			class="input-shadow package-version"
			style={`background-color:${props.backgroundColor}`}
		>
			<div class="cont package-version-name">
				<StabilityIndicator stability={version.stability} />
				{name}
			</div>
			<div
				class="cont package-version-mc-versions"
				style={`${smallFontSize ? "font-size: 0.8rem" : ""}`}
			>
				{versions}
			</div>
			<div class="cont package-version-labels">
				<PackageLabels categories={[]} loaders={loaders} small />
			</div>
		</div>
	);
}

interface PackageVersionEntryProps {
	version: PackageVersion;
	backgroundColor: string;
}

function StabilityIndicator(props: { stability?: "stable" | "latest" }) {
	let letter =
		props.stability == undefined
			? "U"
			: props.stability == "stable"
			? "S"
			: "D";

	let className =
		props.stability == undefined
			? "unknown"
			: props.stability == "stable"
			? "stable"
			: "development";

	let backgroundColor =
		props.stability == undefined
			? "var(--bg)"
			: props.stability == "stable"
			? "var(--instancebg)"
			: "var(--bg)";

	let color =
		props.stability == undefined
			? "var(--bg4)"
			: props.stability == "stable"
			? "var(--instance)"
			: "var(--warning)";

	let tip =
		props.stability == undefined
			? "Unknown stability"
			: props.stability == "stable"
			? "Stable version"
			: "Unstable / development version";

	return (
		<Tip tip={tip} side="top">
			<div
				class={`cont package-version-stability-indicator ${className}`}
				style={`background-color:${backgroundColor};border-color:${color};color:${color}`}
			>
				{letter}
			</div>
		</Tip>
	);
}
