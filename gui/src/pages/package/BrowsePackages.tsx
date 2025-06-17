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
import { PackageMeta, PackageProperties } from "../../types";
import SearchBar from "../../components/input/SearchBar";
import { parseQueryString } from "../../utils";
import InlineSelect from "../../components/input/InlineSelect";
import { FooterData } from "../../App";
import { FooterMode } from "../../components/launch/Footer";
import { errorToast, warningToast } from "../../components/dialog/Toasts";
import PackageLabels from "../../components/package/PackageLabels";
import { PackageType, RepoInfo } from "../../package";
import PackageFilters from "../../components/package/PackageFilters";
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

	let [page, setPage] = createSignal(+params.page);
	let [search, setSearch] = createSignal(searchParams["search"]);
	let [repo, setRepo] = createSignal(searchParams["repo"]);

	let [filteredPackageType, setFilteredPackageType] = createSignal<PackageType>(
		searchParams["package_type"] == undefined
			? "mod"
			: (searchParams["package_type"] as PackageType)
	);
	let [filteredMinecraftVersions, setFilteredMinecraftVersions] = createSignal<
		string[]
	>([]);
	let [filteredLoaders, setFilteredLoaders] = createSignal<string[]>([]);
	let [filteredStability, setFilteredStability] = createSignal<
		"stable" | "latest" | undefined
	>();

	// Updates the URL with current search / filters
	let updateUrl = () => {
		let query = search() == undefined ? "" : `&search=${search()}`;
		window.history.replaceState(
			"",
			"",
			`/packages/${page()}?repo=${selectedRepo()}&package_type=${filteredPackageType()}${query}`
		);
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
			if (repos()!.some((x) => x.id == "std")) {
				return "std";
			}
			return undefined;
		}
		return repo();
	};

	async function updatePackages() {
		let repos: RepoInfo[] = [];
		try {
			repos = await invoke("get_package_repos");
		} catch (e) {
			errorToast("Failed to get available repos: " + e);
			return undefined;
		}

		if (repos.length == 0) {
			warningToast("No repositories available");
		}

		let index = repos.findIndex((x) => x.id == "core");
		if (index != -1) {
			repos.splice(index, 1);
		}
		setRepos(repos);

		try {
			let [packagesToRequest, packageCount] = (await invoke("get_packages", {
				repo: selectedRepo(),
				page: page(),
				search: search(),
				packageKinds: [filteredPackageType()],
			})) as [string[], number];

			setPackageCount(packageCount);

			let promises = [];

			try {
				await invoke("preload_packages", {
					packages: packagesToRequest,
					repo: selectedRepo(),
				});
			} catch (e) {
				errorToast("Failed to load packages: " + e);
			}

			for (let pkg of packagesToRequest) {
				promises.push(
					(async () => {
						try {
							let meta = await invoke("get_package_meta", { package: pkg });
							let props = await invoke("get_package_props", {
								package: pkg,
							});
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
					| string
				)[];
				let packagesAndIds = finalPackages.map((val, i) => {
					if (val == "error") {
						return "error";
					} else {
						let [meta, props] = val;
						return {
							id: packagesToRequest[i],
							meta: meta,
							props: props,
						} as PackageData;
					}
				});
				return packagesAndIds;
			} catch (e) {
				errorToast("Failed to load some packages: " + e);
			}
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
						setPackageType={(type) => {
							setFilteredPackageType(type);
							updateFilters();
						}}
						setMinecraftVersions={setFilteredMinecraftVersions}
						setLoaders={setFilteredLoaders}
						setStability={setFilteredStability}
						filteringVersions={false}
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
														window.location.href = `/packages/package/${data.id}`;
													},
												});
											}}
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
					window.location.href = `/packages/package/${props.id}`;
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
}

export interface BrowsePackagesProps {
	setFooterData: (data: FooterData) => void;
}
