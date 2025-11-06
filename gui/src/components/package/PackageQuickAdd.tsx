import { createResource, createSignal, For, Match, Switch } from "solid-js";
import RepoSelector from "./RepoSelector";
import { canonicalizeListOrSingle } from "../../utils/values";
import {
	getPackageTypeColor,
	getPackageTypeDisplayName,
	getPackageTypeIcon,
	Loader,
	PackageType,
} from "../../package";
import { errorToast } from "../dialog/Toasts";
import { searchPackages } from "../../utils/package";
import "./PackageQuickAdd.css";
import SearchBar from "../input/text/SearchBar";
import { parsePkgRequest } from "../../utils";
import { invoke } from "@tauri-apps/api/core";
import { PackageMeta, PackageProperties } from "../../types";
import LoadingSpinner from "../utility/LoadingSpinner";
import { marked } from "marked";
import InlineSelect from "../input/select/InlineSelect";
import Icon from "../Icon";
import { Plus } from "../../icons";
import IconTextButton from "../input/button/IconTextButton";

export default function PackageQuickAdd(props: PackageQuickAddProps) {
	let [search, setSearch] = createSignal<string | undefined>(undefined);
	let [repo, setRepo] = createSignal<string | undefined>();
	let [finalRepo, setFinalRepo] = createSignal<string | undefined>();
	let [packageType, setPackageType] = createSignal<PackageType>("mod");

	let [packages, packageMethods] = createResource(
		() => finalRepo(),
		async () => {
			if (finalRepo() == undefined) {
				return [];
			}

			let loaders =
				packageType() == "resource_pack" || packageType() == "shader"
					? []
					: canonicalizeListOrSingle(props.loader);

			try {
				let result = await searchPackages(
					finalRepo(),
					0,
					search(),
					[packageType()],
					canonicalizeListOrSingle(props.version),
					loaders,
					[]
				);

				if (result != undefined) {
					return result.packages;
				} else {
					return [];
				}
			} catch (e) {
				errorToast("" + e);
				return [];
			}
		},
		{ initialValue: [] }
	);

	let [previewedPackage, setPreviewedPackage] = createSignal<
		string | undefined
	>();

	let [longDescription, setLongDescription] = createSignal("");

	let [previewedPackageData, _] = createResource(
		() => previewedPackage(),
		async () => {
			if (previewedPackage() == undefined) {
				return undefined;
			}

			try {
				let [meta, props] = (await invoke("get_package_meta_and_props", {
					package: previewedPackage(),
				})) as [PackageMeta, PackageProperties];

				let longDescription =
					meta.long_description == undefined ? "" : meta.long_description;
				let longDescriptionHtml = `<div>${await marked.parse(
					longDescription
				)}</div>`;
				setLongDescription(longDescriptionHtml);

				return [meta, props] as [PackageMeta, PackageProperties];
			} catch (e) {
				errorToast("Failed to get package: " + e);
			}
		}
	);

	return (
		<div class="cont col pop-in-fast package-quick-add">
			<div class="cont col package-quick-add-header">
				<div class="cont start package-quick-add-repos">
					<RepoSelector
						selectedRepo={repo()}
						onSelect={setRepo}
						setFinalSelectedRepo={(x) => {
							setFinalRepo(x);
							packageMethods.refetch();
						}}
						setRepoCategories={() => { }}
						setRepoPackageTypes={() => { }}
						setRepoColor={() => { }}
					/>
				</div>
				<div class="split fullwidth">
					<div class="cont start">
						<InlineSelect
							options={(
								[
									"mod",
									"resource_pack",
									"datapack",
									"plugin",
									"shader",
									"bundle",
								] as PackageType[]
							).map((x) => {
								return {
									value: x,
									contents: (
										<div class="cont">
											<Icon icon={getPackageTypeIcon(x)} size="1rem" />
										</div>
									),
									color: getPackageTypeColor(x),
									tip: getPackageTypeDisplayName(x),
								};
							})}
							selected={packageType()}
							onChange={(x) => {
								setPackageType(x as PackageType);
								packageMethods.refetch();
							}}
							columns={6}
							connected={false}
						/>
					</div>
					<div class="cont end">
						<SearchBar
							value={search()}
							method={(x) => {
								setSearch(x);
								packageMethods.refetch();
							}}
							immediate
						/>
					</div>
				</div>
			</div>
			<div class="package-quick-add-selector">
				<div class="cont col start package-quick-add-options">
					<For each={packages()}>
						{(pkg) => {
							if (pkg == "error") {
								return (
									<div class="package-quick-add-option">
										<div class="cont package-quick-add-option-icon"></div>
										<div class="cont package-quick-add-option-name">Error</div>
									</div>
								);
							}

							let name =
								pkg.meta.name == undefined
									? parsePkgRequest(pkg.id).id
									: pkg.meta.name;

							let icon =
								pkg.meta.icon == undefined
									? "/icons/default_instance.png"
									: pkg.meta.icon;

							let isSelected = () => previewedPackage() == pkg.id;

							return (
								<div
									class={`package-quick-add-option ${isSelected() ? "selected" : ""
										}`}
									onclick={() => {
										setPreviewedPackage(pkg.id);
									}}
								>
									<div class="cont package-quick-add-option-icon">
										<img src={icon} />
									</div>
									<div class="cont package-quick-add-option-name">{name}</div>
								</div>
							);
						}}
					</For>
				</div>
				<div class="package-quick-add-preview">
					<Switch>
						<Match when={previewedPackageData.loading}>
							<div class="cont col fullwidth fullheight">
								<LoadingSpinner size="3rem" />
							</div>
						</Match>
						<Match when={previewedPackageData.error}>
							<div class="cont col fullwidth fullheight">Error</div>
						</Match>
						<Match when={previewedPackage() == undefined}>
							<div class="cont col fullwidth fullheight">
								No package selected
							</div>
						</Match>
						<Match when={previewedPackageData() != undefined}>
							<div class="split3" style="margin-bottom:0.5rem">
								<div>
									<IconTextButton
										icon={Plus}
										size="1.2rem"
										text="Add"
										color="var(--package)"
										bgColor="var(--packagebg)"
										onClick={() => {
											props.onAdd(previewedPackage()!);
										}}
										shadow={false}
									/>
								</div>
								<div class="cont package-quick-add-preview-header">
									{previewedPackageData()![0].name == undefined
										? previewedPackage()
										: previewedPackageData()![0].name!}
								</div>
								<div></div>
							</div>
							<div class="cont package-quick-add-preview-short-description">
								{previewedPackageData()![0].description == undefined
									? ""
									: previewedPackageData()![0].description!}
							</div>
							<hr />
							<div
								class="cont col package-quick-add-preview-long-description package-description"
								innerHTML={longDescription()}
							></div>
						</Match>
					</Switch>
				</div>
			</div>
		</div>
	);
}

export interface PackageQuickAddProps {
	onAdd: (pkg: string) => void;
	version?: string;
	loader?: Loader;
}
