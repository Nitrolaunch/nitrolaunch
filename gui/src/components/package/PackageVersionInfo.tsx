import { createResource, createSignal, For, JSX, Show } from "solid-js";
import {
	Delete,
	Diagram,
	Download,
	Error,
	Folder,
	Link,
	Minecraft,
	Star,
} from "../../icons";
import { PackageVersion } from "../../package";
import { PackageMeta } from "../../types";
import Modal from "../dialog/Modal";
import IconTextButton from "../input/IconTextButton";
import "./PackageVersionInfo.css";
import { StabilityIndicator } from "./PackageVersions";
import { canonicalizeListOrSingle } from "../../utils/values";
import { invoke } from "@tauri-apps/api";
import Icon, { HasWidthHeight } from "../Icon";

export default function PackageVersionInfo(props: PackageVersionInfoProps) {
	let dependencies = () =>
		props.version.relations == undefined
			? []
			: canonicalizeListOrSingle(props.version.relations.dependencies);
	let explicitDependencies = () =>
		props.version.relations == undefined
			? []
			: canonicalizeListOrSingle(props.version.relations.explicit_dependencies);
	let conflicts = () =>
		props.version.relations == undefined
			? []
			: canonicalizeListOrSingle(props.version.relations.conflicts);
	let extensions = () =>
		props.version.relations == undefined
			? []
			: canonicalizeListOrSingle(props.version.relations.extensions);
	let bundled = () =>
		props.version.relations == undefined
			? []
			: canonicalizeListOrSingle(props.version.relations.bundled);
	let compats = () =>
		props.version.relations == undefined
			? []
			: canonicalizeListOrSingle(props.version.relations.compats);
	let recommendations = () =>
		props.version.relations == undefined
			? []
			: canonicalizeListOrSingle(props.version.relations.recommendations);

	let [packageMetas, _] = createResource(
		() => props.version,
		async () => {
			if (props.version == undefined) {
				return undefined;
			}

			let allPackages = new Set();
			for (let pkg of dependencies()
				.concat(explicitDependencies())
				.concat(conflicts())
				.concat(extensions())
				.concat(bundled())
				.concat(recommendations().map((x) => x.value))) {
				allPackages.add(pkg);
			}

			console.log(allPackages);

			let promises = [];
			for (let pkg of allPackages) {
				promises.push(
					(async () => {
						try {
							return [pkg, await invoke("get_package_meta", { package: pkg })];
						} catch (e) {
							console.error(e);
							return "error";
						}
					})()
				);
			}

			let out: { [id: string]: PackageMeta } = {};
			for (let result of (await Promise.all(promises)) as (
				| [string, PackageMeta]
				| "error"
			)[]) {
				if (result != "error") {
					out[result[0]] = result[1];
				}
			}

			return out;
		}
	);

	return (
		<Modal width="50rem" visible={props.visible} onClose={props.onClose}>
			<div class="cont col" id="package-version-info">
				<div class="cont" id="package-version-info-header">
					<StabilityIndicator stability={props.version.stability} />
					<div id="package-version-info-name">
						{props.version.name == undefined
							? props.version.id
							: props.version.name}
					</div>
				</div>
				<div
					class="cont col"
					id="package-version-info-relation-sections-container"
				>
					<Show
						when={
							packageMetas() != undefined &&
							props.version != undefined &&
							!packageMetas.loading
						}
					>
						<RelationList
							header="Dependencies"
							icon={Diagram}
							packages={dependencies()}
							meta={packageMetas()!}
						/>
						<RelationList
							header="Explicit Dependencies"
							icon={Minecraft}
							packages={explicitDependencies()}
							meta={packageMetas()!}
						/>
						<RelationList
							header="Conflicts"
							icon={Error}
							packages={conflicts()}
							meta={packageMetas()!}
						/>
						<RelationList
							header="Extensions"
							icon={Link}
							packages={extensions()}
							meta={packageMetas()!}
						/>
						<RelationList
							header="Bundled"
							icon={Folder}
							packages={bundled()}
							meta={packageMetas()!}
						/>
						<RelationList
							header="Recommended"
							icon={Star}
							packages={recommendations()}
							meta={packageMetas()!}
						/>
					</Show>
				</div>
				<div class="cont">
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
							props.onInstall(props.version.name!);
							props.onClose();
						}}
						text="Install"
					/>
				</div>
			</div>
		</Modal>
	);
}

export interface PackageVersionInfoProps {
	visible: boolean;
	version: PackageVersion;
	onClose: () => void;
	onInstall: (version: string) => void;
}

// A list of relations, like dependencies or
function RelationList(props: RelationListProps) {
	return (
		<Show when={props.packages.length > 0}>
			<div class="cont col package-version-info-relations-container">
				<div class="cont package-version-info-relations-header">
					<Icon icon={props.icon} size="1.2rem" />
					{props.header}
				</div>
				<div class="cont package-version-info-relations">
					<For each={props.packages}>
						{(pkg) => {
							let id = typeof pkg == "object" ? pkg.value : pkg;
							if (!(id in props.meta)) {
								return (
									<div
										class="cont package-version-info-relation"
										style="border-color: var(--error);cursor:initial"
									>
										<img
											src="/icons/default_instance.png"
											class="package-version-info-relation-icon"
											onerror={(e) =>
												((e.target as any).src = "/icons/default_instance.png")
											}
										/>
										Error
									</div>
								);
							}

							let meta = props.meta[id];
							let icon =
								meta.icon == undefined
									? "/icons/default_instance.png"
									: meta.icon;
							let name = meta.name == undefined ? id : meta.name;

							return (
								<div
									class="cont package-version-info-relation"
									onclick={() => {
										window.location.href = `/packages/package/${id}`;
									}}
								>
									<img
										src={icon}
										class="package-version-info-relation-icon"
										onerror={(e) =>
											((e.target as any).src = "/icons/default_instance.png")
										}
									/>
									{name}
								</div>
							);
						}}
					</For>
				</div>
			</div>
		</Show>
	);
}

interface RelationListProps {
	header: string;
	icon: (props: HasWidthHeight) => JSX.Element;
	packages: (string | { value: string; invert?: boolean })[];
	meta: { [id: string]: PackageMeta };
}
