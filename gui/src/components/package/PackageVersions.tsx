import { createResource, createSignal, For, Show } from "solid-js";
import { PackageProperties } from "../../types";
import "./PackageVersions.css";
import { invoke } from "@tauri-apps/api";
import {
	DeclarativePackage,
	PackageAddon,
	PackageVersion,
} from "../../package";
import PackageLabels from "./PackageLabels";
import Tip from "../dialog/Tip";

export default function PackageVersions(props: PackageVersionsProps) {
	let [isScriptPackage, setIsScriptPackage] = createSignal(false);

	let [versions, _] = createResource(async () => {
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

		for (let addonId of Object.keys(declarativeContents.addons)) {
			let addon = declarativeContents.addons[addonId];
			let packageAddon: PackageAddon = { id: addonId, kind: addon.kind };
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
					modloaders: version.modloaders,
					plugin_loaders: version.plugin_loaders,
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
	});

	return (
		<div class="cont col package-versions">
			<Show when={versions() != undefined}>
				<For each={versions()}>
					{(version) => (
						<PackageVersionEntry
							version={version}
							backgroundColor={props.backgroundColor}
						/>
					)}
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

	let minecraftVersions = Array.isArray(version.minecraft_versions)
		? version.minecraft_versions
		: [version.minecraft_versions!];

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
						return (
							<div>
								{/* {i() == minecraftVersions.length - 1 ? version : `${version}, `} */}
								{version}
							</div>
						);
					}
				}}
			</For>
		</Show>
	);

	let modloaders =
		version.modloaders == undefined
			? []
			: Array.isArray(version.modloaders)
			? version.modloaders
			: [version.modloaders];

	let pluginLoaders =
		version.plugin_loaders == undefined
			? []
			: Array.isArray(version.plugin_loaders)
			? version.plugin_loaders
			: [version.plugin_loaders];

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
				<PackageLabels
					categories={[]}
					loaders={modloaders.concat(pluginLoaders)}
					small
				/>
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
				class="cont package-version-stability-indicator"
				style={`background-color:${backgroundColor};border-color:${color};color:${color}`}
			>
				{letter}
			</div>
		</Tip>
	);
}
