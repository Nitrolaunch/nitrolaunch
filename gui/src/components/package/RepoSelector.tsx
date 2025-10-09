import { errorToast, warningToast } from "../dialog/Toasts";
import { PackageCategory, PackageType, RepoInfo } from "../../package";
import { invoke } from "@tauri-apps/api";
import { createSignal, Show, createResource, createEffect } from "solid-js";
import InlineSelect from "../input/select/InlineSelect";
import Icon from "../Icon";
import { Home } from "../../icons";

export default function RepoSelector(props: RepoSelectorProps) {
	let [repos, setRepos] = createSignal<RepoInfo[] | undefined>();
	let [lastSelectedRepo, setLastSelectedRepo] = createSignal<
		string | undefined
	>();

	let selectedRepo = () => {
		if (repos() == undefined) {
			return undefined;
		}
		if (props.selectedRepo == undefined) {
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
		return props.selectedRepo;
	};

	let [result, __] = createResource(async () => {
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
	});

	createEffect(() => {
		if (repos() != undefined) {
			let repo = repos()!.find((x) => x.id == selectedRepo());
			if (repo == undefined) {
				props.setRepoPackageTypes(undefined);
				props.setRepoCategories(undefined);
				props.setRepoColor(undefined);
			} else {
				props.setRepoPackageTypes(
					repo.meta.package_types == undefined ||
						repo.meta.package_types.length == 0
						? undefined
						: repo.meta.package_types!
				);

				props.setRepoCategories(
					repo.meta.package_categories == undefined ||
						repo.meta.package_categories.length == 0
						? undefined
						: repo.meta.package_categories!
				);

				props.setRepoColor(repo.meta.color);
			}

			props.setFinalSelectedRepo(selectedRepo());
		}
	});

	return (
		<Show when={!result.loading}>
			<InlineSelect
				options={repos()!.map((x) => {
					if (x.id == "std") {
						return {
							value: "std",
							contents: <Icon icon={Home} size="1rem" />,
						};
					}
					return {
						value: x.id,
						contents: (
							<div style="padding:0rem 0.3rem">
								{x.meta.name == undefined
									? x.id.replace(/\_/g, " ").toLocaleUpperCase()
									: x.meta.name.toLocaleUpperCase()}
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
						props.onSelect(x);
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
	);
}

export interface RepoSelectorProps {
	selectedRepo: string | undefined;
	onSelect: (repo: string) => void;
	// The final value for the selected repository after processing
	setFinalSelectedRepo: (repo: string | undefined) => void;
	setRepoPackageTypes: (value: PackageType[] | undefined) => void;
	setRepoCategories: (value: PackageCategory[] | undefined) => void;
	setRepoColor: (value: string | undefined) => void;
}
