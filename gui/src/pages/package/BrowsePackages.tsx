import { useLocation, useNavigate, useParams } from "@solidjs/router";
import "./BrowsePackages.css";
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
import "@thisbeyond/solid-select/style.css";
import PageButtons from "../../components/input/button/PageButtons";
import { PackageMeta } from "../../types";
import SearchBar from "../../components/input/text/SearchBar";
import { formatNumber, parseQueryString } from "../../utils";
import { FooterData } from "../../App";
import { FooterMode } from "../../components/navigation/Footer";
import { errorToast } from "../../components/dialog/Toasts";
import PackageLabels from "../../components/package/PackageLabels";
import { Loader, PackageCategory, PackageType } from "../../package";
import PackageFilters, {
	defaultPackageFilters,
	PackageFilterOptions,
} from "../../components/package/PackageFilters";
import LoadingSpinner from "../../components/utility/LoadingSpinner";
import RepoSelector from "../../components/package/RepoSelector";
import { searchPackages } from "../../utils/package";
import Tip from "../../components/dialog/Tip";
import IconTextButton from "../../components/input/button/IconTextButton";
import { Download, Grid, Menu, Properties, Refresh } from "../../icons";
import { invoke } from "@tauri-apps/api/core";
import { loadPagePlugins } from "../../plugins";
import Icon from "../../components/Icon";
import ViewPackage from "./ViewPackage";
import IconButton from "../../components/input/button/IconButton";

const PACKAGES_PER_PAGE = 12;

export default function BrowsePackages(props: BrowsePackagesProps) {
	let navigate = useNavigate();

	let params = useParams();
	let searchParams = parseQueryString(useLocation().search);

	let [isAlternate, setIsAlternate] = createSignal(true);

	onMount(() => loadPagePlugins("packages"));

	createEffect(() => {
		if (!isAlternate()) {
			props.setFooterData({
				mode: FooterMode.PreviewPackage,
				selectedItem: undefined,
				action: () => {},
			});
		}
	});

	// Filters and other browse functions

	let filters = () => {
		if (searchParams["filters"] == undefined) {
			return defaultPackageFilters();
		}

		try {
			return JSON.parse(
				decodeURIComponent(searchParams["filters"]),
			) as PackageFilterOptions;
		} catch (e) {
			console.error("Failed to parse filters: " + e);
			return defaultPackageFilters();
		}
	};

	let [page, setPage] = createSignal(+params.page);
	let [search, setSearch] = createSignal(searchParams["search"]);
	let [repo, setRepo] = createSignal(searchParams["repo"]);

	let [selectedRepo, setSelectedRepo] = createSignal<string | undefined>();

	let [filteredPackageType, setFilteredPackageType] = createSignal<PackageType>(
		searchParams["package_type"] == undefined
			? "mod"
			: (searchParams["package_type"] as PackageType),
	);
	let [filteredMinecraftVersions, setFilteredMinecraftVersions] = createSignal<
		string[]
	>(filters().minecraft_versions);
	let [filteredLoaders, setFilteredLoaders] = createSignal<string[]>(
		filters().loaders,
	);
	let [filteredCategories, setFilteredCategories] = createSignal<
		PackageCategory[]
	>(filters().categories);
	let [filteredStability, setFilteredStability] = createSignal<
		"stable" | "latest" | undefined
	>();

	// Creates the PackageFilterOptions object to be put in URL parameters
	let createPackageFiltersObject = () => {
		return {
			minecraft_versions: filteredMinecraftVersions(),
			loaders: filteredLoaders(),
			categories: filteredCategories(),
		} as PackageFilterOptions;
	};

	// Updates the URL with current search / filters
	let updateUrl = () => {
		let url = getBrowseUrl(
			page(),
			selectedRepo(),
			filteredPackageType(),
			search(),
			createPackageFiltersObject(),
		);
		window.history.replaceState("", "", url);
	};

	// Refetches packages and modifies the URL
	let updateFilters = () => {
		refetchPackages();
		updateUrl();
	};

	// Packages and repos
	let [packages, packageMethods] = createResource(
		() => selectedRepo(),
		updatePackages,
	);
	let refetchPackages = () => {
		packageMethods.mutate(undefined);
		packageMethods.refetch();
	};

	let [packageCount, setPackageCount] = createSignal(0);

	let [repoPackageTypes, setRepoPackageTypes] = createSignal<
		PackageType[] | undefined
	>();
	let [repoCategories, setRepoCategories] = createSignal<
		PackageCategory[] | undefined
	>();

	async function updatePackages() {
		if (selectedRepo() == undefined) {
			return undefined;
		}

		try {
			let result = await searchPackages(
				selectedRepo(),
				page(),
				search(),
				[filteredPackageType()],
				filteredMinecraftVersions(),
				filteredLoaders() as Loader[],
				filteredCategories(),
			);

			if (result != undefined) {
				setPackageCount(result.totalCount);
				return result.packages;
			}
		} catch (e) {
			errorToast(`${e}`);
		}
	}

	let [selectedPackage, setSelectedPackage] = createSignal<string | undefined>(
		undefined,
	);

	// Placeholder when packages are loading
	let packagePlaceholders = () => (
		<For each={Array.from({ length: PACKAGES_PER_PAGE })}>
			{() => (
				<div class={`cont package ${isAlternate() ? "alternate" : ""}`}>
					<LoadingSpinner size={isAlternate() ? "2rem" : "3rem"} />
				</div>
			)}
		</For>
	);

	return (
		<div class="cont col fullwidth">
			<div class="cont col" id="browse-packages">
				<div id="browse-header">
					<div class="cont">
						<RepoSelector
							selectedRepo={repo()}
							onSelect={(x) => {
								setRepo(x);
								setPage(0);
								updateFilters();
							}}
							setFinalSelectedRepo={setSelectedRepo}
							setRepoPackageTypes={setRepoPackageTypes}
							setRepoCategories={setRepoCategories}
							setRepoColor={() => {}}
						/>
					</div>
					<div></div>
					<div class="cont" style="justify-content:flex-end">
						<div class="cont">
							<IconButton
								icon={Grid}
								size="1.5rem"
								onClick={() => setIsAlternate(false)}
								color="transparent"
								border={isAlternate() ? undefined : "var(--fg2)"}
								iconColor={isAlternate() ? undefined : "var(--fg2)"}
							/>
							<IconButton
								icon={Menu}
								size="1.5rem"
								onClick={() => setIsAlternate(true)}
								color="transparent"
								border={isAlternate() ? "var(--fg2)" : undefined}
								iconColor={isAlternate() ? "var(--fg2)" : undefined}
							/>
							<IconButton
								icon={Properties}
								size="1.5rem"
								onClick={() => {}}
								color="transparent"
								border={undefined}
								iconColor={undefined}
							/>
						</div>
						<SearchBar
							placeholder="Search for packages..."
							value={
								search() == undefined
									? undefined
									: decodeURIComponent(search()!)
							}
							method={(term) => {
								setSearch(term);
								setPage(0);
								updateFilters();
							}}
						/>
					</div>
				</div>
				{/* <Show when={repoColor() != undefined}>
					<div id="browse-gradient" style={`--repo-color:${repoColor()}`}></div>
				</Show> */}
				<div></div>
				<div id="browse-subheader">
					<PackageFilters
						packageType={filteredPackageType()}
						minecraftVersions={filteredMinecraftVersions()}
						loaders={filteredLoaders()}
						stability={filteredStability()}
						categories={filteredCategories()}
						setPackageType={(type) => {
							setFilteredPackageType(type);
							setPage(0);
							updateFilters();
						}}
						setMinecraftVersions={(versions) => {
							setFilteredMinecraftVersions(versions);
							updateFilters();
						}}
						setLoaders={(loaders) => {
							setFilteredLoaders(loaders);
							updateFilters();
						}}
						setCategories={(categories) => {
							setFilteredCategories(categories);
							updateFilters();
						}}
						setStability={setFilteredStability}
						availablePackageTypes={repoPackageTypes()}
						filteringVersions={false}
						features={[]}
						setFeatures={() => {}}
						availableCategories={repoCategories()}
					/>
				</div>
				<Show when={!isAlternate()}>
					<div class="split3 fullwidth">
						<div></div>
						<PageButtons
							page={page()}
							pageCount={Math.floor(packageCount() / PACKAGES_PER_PAGE)}
							pageFunction={(page) => {
								setPage(page);
								updateFilters();
							}}
						/>
						<div class="cont end">
							<Tip
								tip="Refetches packages and their new versions"
								side="left"
								zIndex="10"
							>
								<IconTextButton
									icon={Refresh}
									text="Sync Packages"
									size="1.5rem"
									color="var(--package)"
									bgColor="var(--packagebg)"
									onClick={async () => {
										try {
											await invoke("sync_packages");
										} catch (e) {
											errorToast("Failed to sync packages: " + e);
										}
									}}
								/>
							</Tip>
						</div>
					</div>
				</Show>
				<div id="browse-container" class={isAlternate() ? "alternate" : ""}>
					<div id="packages-container" class={isAlternate() ? "alternate" : ""}>
						<Show
							when={packages() != undefined}
							fallback={packagePlaceholders()}
						>
							<For each={packages()}>
								{(data) => {
									if (data == "error") {
										return (
											<div class="cont package package-error">
												Error with package
											</div>
										);
									} else {
										return (
											<Package
												id={data.id}
												meta={data.meta}
												selected={selectedPackage()}
												onSelect={(pkg) => {
													setSelectedPackage(pkg);
													let url = `/packages/package/${
														data.id
													}?filters=${JSON.stringify(
														createPackageFiltersObject(),
													)}`;

													if (!isAlternate()) {
														props.setFooterData({
															mode: FooterMode.PreviewPackage,
															selectedItem: "",
															action: () => {
																navigate(url);
															},
															selectedPackageGallery: data.meta.gallery,
														});
													}
												}}
												getPackageFiltersObject={createPackageFiltersObject}
												alternate={isAlternate()}
											/>
										);
									}
								}}
							</For>
						</Show>
					</div>
					<Show when={isAlternate()}>
						<div id="package-preview">
							<Switch>
								<Match when={selectedPackage() != undefined}>
									<ViewPackage
										id={selectedPackage()!}
										filters={createPackageFiltersObject()}
										small
										setFooterData={props.setFooterData}
									/>
								</Match>
								<Match when={selectedPackage() == undefined}>
									<div class="cont fullwidth fullheight">
										No package selected
									</div>
								</Match>
							</Switch>
						</div>
					</Show>
				</div>
				<Show when={!isAlternate()}>
					<PageButtons
						page={page()}
						pageCount={Math.floor(packageCount() / PACKAGES_PER_PAGE)}
						pageFunction={(page) => {
							setPage(page);
							updateFilters();
						}}
					/>
				</Show>
				<br />
				<br />
				<br />
			</div>
		</div>
	);
}

function Package(props: PackageProps) {
	let navigate = useNavigate();

	let baseImage = () => {
		if (props.alternate) {
			if (props.meta.icon != undefined) {
				return props.meta.icon;
			} else if (props.meta.banner != undefined) {
				return props.meta.banner;
			}
		} else {
			if (props.meta.banner != undefined) {
				return props.meta.banner;
			} else if (
				props.meta.gallery != undefined &&
				props.meta.gallery!.length > 0
			) {
				return props.meta.gallery[0];
			} else {
				return props.meta.icon;
			}
		}
	};

	let image = () => {
		let base = baseImage();
		if (base == undefined) {
			return "/icons/default_instance.png";
		} else {
			return base;
		}
	};

	let isSelected = () => props.selected == props.id;

	return (
		<div
			class={`cont col ${props.alternate ? "" : "shadow"} package ${
				isSelected() ? "selected" : ""
			} ${props.alternate ? "alternate" : ""}`}
			style="cursor:pointer"
			onclick={() => {
				// Double click to open
				if (isSelected()) {
					navigate(
						`/packages/package/${props.id}?filters=${JSON.stringify(
							props.getPackageFiltersObject(),
						)}`,
					);
				} else {
					props.onSelect(props.id);
				}
			}}
		>
			<div class="package-inner">
				<div class="cont package-image-container">
					<img
						src={image()}
						class="package-image"
						onerror={(e: any) => (e.target.src = "/icons/default_instance.png")}
					/>
					<Show when={!props.alternate}>
						<div class="package-image-gradient"></div>
					</Show>
				</div>
				<div class="cont col package-header">
					<div class="cont start package-name">
						{props.meta.name}
						<Show when={props.meta.downloads != undefined && !props.alternate}>
							<div
								class="cont"
								style="color: var(--fg3);gap:0.2rem;font-size:0.95rem"
							>
								<Icon icon={Download} size="1rem" />
								{formatNumber(props.meta.downloads!)}
							</div>
						</Show>
					</div>
					<Show when={props.meta.categories != undefined}>
						<div style="margin-top:-0.2rem">
							<PackageLabels
								categories={props.meta.categories!}
								loaders={[]}
								packageTypes={[]}
								small
								limit={3}
							/>
						</div>
					</Show>
					<Show when={!props.alternate}>
						<div class="browse-package-description">
							{props.meta.description}
						</div>
					</Show>
				</div>
			</div>
		</div>
	);
}

interface PackageProps {
	id: string;
	meta: PackageMeta;
	selected?: string;
	onSelect: (pkg: string) => void;
	getPackageFiltersObject: () => PackageFilterOptions;
	alternate: boolean;
}

export interface BrowsePackagesProps {
	setFooterData: (data: FooterData) => void;
}

export function getBrowseUrl(
	page: number,
	repo: string | undefined,
	packageType: PackageType,
	search: string | undefined,
	filters: PackageFilterOptions,
) {
	let query = search == undefined ? "" : `&search=${search}`;
	let filters2 = JSON.stringify(filters);
	let repo2 = repo == undefined ? "" : `&repo=${repo}`;
	return `/packages/${page}?package_type=${packageType}${repo2}${query}&filters=${filters2}`;
}
