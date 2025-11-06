import { invoke } from "@tauri-apps/api/core";
import { Loader, PackageCategory, PackageType } from "../package";
import { PackageMeta, PackageProperties, PackageSearchResults } from "../types";
import { parsePkgRequest, pkgRequestToString } from "../utils";

export async function searchPackages(
	repo: string | undefined,
	page: number,
	search: string | undefined,
	packageKinds: PackageType[],
	minecraftVersions: string[],
	loaders: Loader[],
	categories: PackageCategory[]
): Promise<ExpandedPackageSearchResults | undefined> {
	try {
		let results: PackageSearchResults = await invoke("get_packages", {
			repo: repo,
			page: page,
			search: search,
			packageKinds: packageKinds,
			minecraftVersions: minecraftVersions,
			loaders: loaders,
			categories: categories,
		});

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
					repo: repo,
				});
			} catch (e) {
				throw "Failed to load packages: " + e;
			}

			for (let pkg of results.results) {
				promises.push(
					(async () => {
						try {
							let [meta, props] = (await invoke("get_package_meta_and_props", {
								package: pkg,
							})) as [PackageMeta, PackageProperties];
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
					| [PackageMeta, PackageProperties]
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
				throw "Failed to load some packages: " + e;
			}
		}

		return {
			packages: packages,
			totalCount: results.total_results,
		};
	} catch (e) {
		throw "Failed to search packages: " + e;
	}
}

export interface ExpandedPackageSearchResults {
	totalCount: number;
	packages: (PackageData | "error")[];
}

export interface PackageData {
	id: string;
	meta: PackageMeta;
	props: PackageProperties;
}
