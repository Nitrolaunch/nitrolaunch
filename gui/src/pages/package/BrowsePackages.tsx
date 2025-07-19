import { useLocation, useParams } from "@solidjs/router";
import "./BrowsePackages.css";
import { invoke } from "@tauri-apps/api";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	Show,
} from "solid-js";
import "@thisbeyond/solid-select/style.css";
import PageButtons from "../../components/input/PageButtons";
import {
	PackageMeta,
	PackageProperties,
	PackageSearchResults,
} from "../../types";
import SearchBar from "../../components/input/SearchBar";
import {
	parsePkgRequest,
	parseQueryString,
	pkgRequestToString,
} from "../../utils";
import InlineSelect from "../../components/input/InlineSelect";
import { FooterData } from "../../App";
import { FooterMode } from "../../components/navigation/Footer";
import { errorToast, warningToast } from "../../components/dialog/Toasts";
import PackageLabels from "../../components/package/PackageLabels";
import { PackageCategory, PackageType, RepoInfo } from "../../package";
import PackageFilters, {
	defaultPackageFilters,
	PackageFilterOptions,
} from "../../components/package/PackageFilters";
import LoadingSpinner from "../../components/utility/LoadingSpinner";

const PACKAGES_PER_PAGE = 12;

export default function BrowsePackages(props: BrowsePackagesProps) {
	let params = useParams();
	let searchParams = parseQueryString(useLocation().search);

	createEffect(() => {
		props.setFooterData({
			mode: FooterMode.PreviewPackage,
			selectedItem: undefined,
			action: () => {},
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
	let [lastSelectedRepo, setLastSelectedRepo] = createSignal<
		string | undefined
	>(undefined);

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
		let query = search() == undefined ? "" : `&search=${search()}`;
		let filters = JSON.stringify(createPackageFiltersObject());
		let url = `/packages/${page()}?repo=${selectedRepo()}&package_type=${filteredPackageType()}${query}&filters=${filters}`;
		window.history.replaceState("", "", url);
	};

	// Refetches packages and modifies the URL
	let updateFilters = () => {
		refetchPackages();
		updateUrl();
	};

	// Packages and repos
	let [packages, packageMethods] = createResource(updatePackages);
	let refetchPackages = () => {
		packageMethods.mutate(undefined);
		packageMethods.refetch();
	};

	let [repos, setRepos] = createSignal<RepoInfo[] | undefined>(undefined);
	let [packageCount, setPackageCount] = createSignal(0);

	let selectedRepo = () => {
		if (repos() == undefined) {
			return undefined;
		}
		if (repo() == undefined) {
			if (lastSelectedRepo() != undefined) {
				if (repos()!.some((x) => x.id == lastSelectedRepo())) {
					return lastSelectedRepo();
				}
			}
			if (repos()!.some((x) => x.id == "std")) {
				return "std";
			}
			return undefined;
		}
		return repo();
	};

	let repoPackageTypes = () => {
		if (repos() == undefined || selectedRepo() == undefined) {
			return undefined;
		}
		let repo = repos()!.find((x) => x.id == selectedRepo())!;

		return repo.meta.package_types == undefined ||
			repo.meta.package_types.length == 0
			? undefined
			: repo.meta.package_types!;
	};

	async function updatePackages() {
		let repos: RepoInfo[] = [];
		let lastSelectedRepo: string | undefined = undefined;
		try {
			[repos, lastSelectedRepo] = (await Promise.all([
				invoke("get_package_repos"),
				invoke("get_last_selected_repo"),
			])) as [RepoInfo[], string | undefined];
		} catch (e) {
			errorToast("Failed to get available repos: " + e);
			return undefined;
		}

		if (repos.length == 0) {
			warningToast("No repositories available");
		}

		// Remove the core repository
		let index = repos.findIndex((x) => x.id == "core");
		if (index != -1) {
			repos.splice(index, 1);
		}
		setRepos(repos);
		setLastSelectedRepo(lastSelectedRepo);

		try {
			let results: PackageSearchResults = await invoke("get_packages", {
				repo: selectedRepo(),
				page: page(),
				search: search(),
				packageKinds: [filteredPackageType()],
				minecraftVersions: filteredMinecraftVersions(),
				loaders: filteredLoaders(),
				categories: filteredCategories(),
			});
			setPackageCount(results.total_results);

			let packages: (PackageData | "error")[] = [];

			// Fill out results from existing previews, removing them from the list if the preview is present
			for (let i = 0; i < results.results.length; i++) {
				let pkg = parsePkgRequest(results.results[i]);
				let preview =
					pkg.id in results.previews
						? results.previews[pkg.id]
						: pkgRequestToString(pkg) in results.previews
						? results.previews[pkgRequestToString(pkg)]
						: undefined;
				if (preview != undefined) {
					packages.push({
						id: pkgRequestToString(pkg),
						meta: preview[0],
						props: preview[1],
					});
					results.results.splice(i, 1);
					i--;
				}
			}

			// Get the remaining packages
			if (results.results.length > 0) {
				let promises = [];

				try {
					await invoke("preload_packages", {
						packages: results.results,
						repo: selectedRepo(),
					});
				} catch (e) {
					errorToast("Failed to load packages: " + e);
				}

				for (let pkg of results.results) {
					promises.push(
						(async () => {
							try {
								let [meta, props] = (await invoke(
									"get_package_meta_and_props",
									{
										package: pkg,
									}
								)) as [PackageMeta, PackageProperties];
								return [meta, props];
							} catch (e) {
								console.error(e);
								return "error";
							}
						})()
					);
				}

				try {
					let finalPackages = (await Promise.all(promises)) as (
						| [PackageMeta, PackageProps]
						| "error"
					)[];
					let newPackages = finalPackages.map((val, i) => {
						if (val == "error") {
							return "error";
						} else {
							let [meta, props] = val;
							return {
								id: results.results[i],
								meta: meta,
								props: props,
							} as PackageData;
						}
					});

					packages = packages.concat(newPackages);
				} catch (e) {
					errorToast("Failed to load some packages: " + e);
				}
			}

			return packages;
		} catch (e) {
			errorToast("Failed to search packages: " + e);
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
		<div class="cont col">
			<div class="cont col" id="browse-packages">
				<div id="browse-header">
					<div class="cont">
						<Show when={repos() != undefined}>
							<InlineSelect
								options={repos()!.map((x) => {
									return {
										value: x.id,
										contents: (
											<div style="padding:0rem 0.3rem">
												{x.id.replace(/\_/g, " ").toLocaleUpperCase()}
											</div>
										),
										color: x.meta.color,
										selectedTextColor: x.meta.text_color,
									};
								})}
								connected={false}
								grid={true}
								selected={selectedRepo()}
								columns={repos()!.length}
								onChange={(x) => {
									if (x != undefined) {
										setRepo(x);
										setPage(0);
										updateFilters();
										(async () => {
											try {
												await invoke("set_last_selected_repo", { repo: x });
											} catch (e) {}
										})();
									}
								}}
								optionClass="repo"
								solidSelect={true}
							/>
						</Show>
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
					/>
				</div>
				<div class="cont">
					<PageButtons
						page={page()}
						pageCount={Math.floor(packageCount() / PACKAGES_PER_PAGE)}
						pageFunction={(page) => {
							setPage(page);
							updateFilters();
						}}
					/>
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
												props.setFooterData({
													mode: FooterMode.PreviewPackage,
													selectedItem: "",
													action: () => {
														window.location.href = `/packages/package/${
															data.id
														}?filters=${createPackageFiltersObject()}`;
													},
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
			class={`cont col input-shadow package ${isSelected() ? "selected" : ""}`}
			style="cursor:pointer"
			onclick={() => {
				// Double click to open
				if (isSelected()) {
					window.location.href = `/packages/package/${
						props.id
					}?filters=${JSON.stringify(props.getPackageFiltersObject())}`;
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
						onerror={(e) => e.target.remove()}
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

interface PackageData {
	id: string;
	meta: PackageMeta;
	props: PackageProperties;
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
