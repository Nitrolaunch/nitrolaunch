import {
	useLocation,
	useNavigate,
	useParams,
} from "@solidjs/router";
import "./BrowsePackages.css";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	Show,
} from "solid-js";
import "@thisbeyond/solid-select/style.css";
import PageButtons from "../../components/input/PageButtons";
import { PackageMeta } from "../../types";
import SearchBar from "../../components/input/SearchBar";
import { parseQueryString } from "../../utils";
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
import IconTextButton from "../../components/input/IconTextButton";
import { Refresh } from "../../icons";
import { invoke } from "@tauri-apps/api";

const PACKAGES_PER_PAGE = 12;

export default function BrowsePackages(props: BrowsePackagesProps) {
	let navigate = useNavigate();

	let params = useParams();
	let searchParams = parseQueryString(useLocation().search);

	createEffect(() => {
		props.setFooterData({
			mode: FooterMode.PreviewPackage,
			selectedItem: undefined,
			action: () => { },
		});
	});

	// Filters and other browse functions

	let filters = () => {
		if (searchParams["filters"] == undefined) {
			return defaultPackageFilters();
		}

		try {
			return JSON.parse(
				decodeURIComponent(searchParams["filters"])
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
			: (searchParams["package_type"] as PackageType)
	);
	let [filteredMinecraftVersions, setFilteredMinecraftVersions] = createSignal<
		string[]
	>(filters().minecraft_versions);
	let [filteredLoaders, setFilteredLoaders] = createSignal<string[]>(
		filters().loaders
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
			createPackageFiltersObject()
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
		updatePackages
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
	let [repoColor, setRepoColor] = createSignal<string | undefined>();

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
				filteredCategories()
			);

			if (result != undefined) {
				setPackageCount(result.totalCount);
				return result.packages;
			}
		} catch (e) {
			console.log(e);
			errorToast(`${e}`);
		}
	}

	let [selectedPackage, setSelectedPackage] = createSignal<string | undefined>(
		undefined
	);

	// Placeholder when packages are loading
	let packagePlaceholders = () => (
		<For each={Array.from({ length: PACKAGES_PER_PAGE })}>
			{() => (
				<div class="cont package">
					<LoadingSpinner size="3rem" />
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
								console.log(repo());
								setPage(0);
								updateFilters();
							}}
							setFinalSelectedRepo={setSelectedRepo}
							setRepoPackageTypes={setRepoPackageTypes}
							setRepoCategories={setRepoCategories}
							setRepoColor={setRepoColor}
						/>
					</div>
					<h1 class="noselect">Packages</h1>
					<div class="cont" style="justify-content:flex-end">
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
				<Show when={repoColor() != undefined}>
					<div id="browse-gradient" style={`--repo-color:${repoColor()}`}></div>
				</Show>
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
						setFeatures={() => { }}
						availableCategories={repoCategories()}
					/>
				</div>
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
						<Tip tip="Refetches packages and their new versions" side="left">
							<IconTextButton
								icon={Refresh}
								text="Sync Packages"
								size="22px"
								color="var(--bg2)"
								selectedColor="var(--package)"
								selectedBg="var(--bg-1)"
								onClick={async () => {
									try {
										await invoke("sync_packages");
									} catch (e) {
										errorToast("Failed to sync packages: " + e);
									}
								}}
								selected={true}
							/>
						</Tip>
					</div>
				</div>
				<div id="packages-container">
					<Show when={packages() != undefined} fallback={packagePlaceholders()}>
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
												let url = `/packages/package/${data.id
													}?filters=${JSON.stringify(
														createPackageFiltersObject()
													)}`;

												props.setFooterData({
													mode: FooterMode.PreviewPackage,
													selectedItem: "",
													action: () => {
														navigate(url);
													},
													selectedPackageGallery: data.meta.gallery,
												});
											}}
											getPackageFiltersObject={createPackageFiltersObject}
										/>
									);
								}
							}}
						</For>
					</Show>
				</div>
				<PageButtons
					page={page()}
					pageCount={Math.floor(packageCount() / PACKAGES_PER_PAGE)}
					pageFunction={(page) => {
						setPage(page);
						updateFilters();
					}}
				/>
				<br />
				<br />
				<br />
			</div>
		</div>
	);
}

function Package(props: PackageProps) {
	let navigate = useNavigate();

	let image =
		props.meta.banner == undefined
			? props.meta.gallery == undefined || props.meta.gallery!.length == 0
				? props.meta.icon == undefined
					? "/icons/default_instance.png"
					: props.meta.icon
				: props.meta.gallery![0]
			: props.meta.banner;

	let isSelected = () => props.selected == props.id;

	return (
		<div
			class={`cont col input-shadow bubble-hover-small package ${isSelected() ? "selected" : ""}`}
			style="cursor:pointer"
			onclick={() => {
				// Double click to open
				if (isSelected()) {
					navigate(
						`/packages/package/${props.id}?filters=${JSON.stringify(
							props.getPackageFiltersObject()
						)}`
					);
				} else {
					props.onSelect(props.id);
				}
			}}
		>
			<div class="package-inner">
				<div class="package-image-container">
					<img
						src={image}
						class="package-image"
						onerror={(e: any) => (e.target.src = "/icons/default_instance.png")}
					/>
				</div>
				<div class="cont col package-header">
					<div class="package-name">{props.meta.name}</div>
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
					<div class="package-description">{props.meta.description}</div>
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
}

export interface BrowsePackagesProps {
	setFooterData: (data: FooterData) => void;
}

export function getBrowseUrl(
	page: number,
	repo: string | undefined,
	packageType: PackageType,
	search: string | undefined,
	filters: PackageFilterOptions
) {
	let query = search == undefined ? "" : `&search=${search}`;
	let filters2 = JSON.stringify(filters);
	let repo2 = repo == undefined ? "" : `&repo=${repo}`;
	return `/packages/${page}?package_type=${packageType}${repo2}${query}&filters=${filters2}`;
}
