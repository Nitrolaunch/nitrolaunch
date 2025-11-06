import { useLocation, useParams } from "@solidjs/router";
import "./ViewPackage.css";
import { invoke } from "@tauri-apps/api/core";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	JSX,
	Show,
} from "solid-js";
import "@thisbeyond/solid-select/style.css";
import { PackageMeta, PackageProperties } from "../../types";
import { marked } from "marked";
import { errorToast } from "../../components/dialog/Toasts";
import { FooterData } from "../../App";
import { FooterMode } from "../../components/navigation/Footer";
import Icon, { HasWidthHeight } from "../../components/Icon";
import {
	Book,
	CurlyBraces,
	Download,
	Folder,
	Globe,
	Hashtag,
	Heart,
	Key,
	Picture,
	Popout,
	Text,
	User,
	Warning,
} from "../../icons";
import PackageLabels from "../../components/package/PackageLabels";
import { RepoInfo } from "../../package";
import { beautifyString, formatNumber, parsePkgRequest, parseQueryString } from "../../utils";
import PackageVersions from "../../components/package/PackageVersions";
import PackageInstallModal from "../../components/package/PackageInstallModal";
import { canonicalizeListOrSingle } from "../../utils/values";
import LoadingSpinner from "../../components/utility/LoadingSpinner";
import { PackageFilterOptions } from "../../components/package/PackageFilters";
import { open } from "@tauri-apps/plugin-shell";
import IconButton from "../../components/input/button/IconButton";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import PackageGallery from "../../components/package/PackageGallery";

export default function ViewPackage(props: ViewPackageProps) {
	let params = useParams();
	let searchParams = parseQueryString(useLocation().search);

	let packageId = params.id;

	let [meta] = createResource(updateMetaAndProps);
	let [properties, setProperties] = createSignal<PackageProperties | undefined>(
		undefined
	);

	let [repoInfo, setRepoInfo] = createSignal<RepoInfo | undefined>(undefined);
	let [shortDescription, setShortDescription] = createSignal("");
	let [longDescription, setLongDescription] = createSignal("");

	let [selectedTab, setSelectedTab] = createSignal("description");

	let [showInstallModal, setShowInstallModal] = createSignal(false);
	let [installVersion, setInstallVersion] = createSignal<string | undefined>();

	let filters = () => {
		if (searchParams["filters"] == undefined) {
			return undefined;
		}

		try {
			return JSON.parse(
				decodeURIComponent(searchParams["filters"])
			) as PackageFilterOptions;
		} catch (e) {
			console.error("Failed to parse filters: " + e);
			return undefined;
		}
	};

	createEffect(() => {
		props.setFooterData({
			selectedItem: "",
			mode: FooterMode.InstallPackage,
			action: () => setShowInstallModal(true),
		});
	});

	async function updateMetaAndProps() {
		try {
			let [[meta, properties], repos] = (await Promise.all([
				invoke("get_package_meta_and_props", {
					package: packageId,
				}),
				invoke("get_package_repos"),
			])) as [[PackageMeta, PackageProperties], RepoInfo[]];

			let request = parsePkgRequest(packageId);
			if (request.repository != undefined)
				for (let repo of repos) {
					if (repo.id == request.repository) {
						setRepoInfo(repo);
					}
				}

			let description = meta.description == undefined ? "" : meta.description;
			setShortDescription(description.slice(0, 200));
			let longDescription =
				meta.long_description == undefined ? "" : meta.long_description;
			let longDescriptionHtml = `<div>${await marked.parse(
				longDescription
			)}</div>`;
			setLongDescription(longDescriptionHtml);

			setProperties(properties);

			return meta;
		} catch (e) {
			errorToast("Failed to load package: " + e);
			return undefined;
		}
	}

	return (
		<Show
			when={meta() != undefined && properties() != undefined}
			fallback={
				<div class="cont" style="width:100%">
					<LoadingSpinner size="5rem" />
				</div>
			}
		>
			<div class="cont col" style="width:100%">
				<Show when={meta()!.banner != undefined}>
					<div id="package-banner-container">
						<img
							src={meta()!.banner}
							id="package-banner"
							onerror={(e) => e.target.remove()}
						/>
						<div id="package-banner-gradient"></div>
					</div>
				</Show>
				<div class="cont col" id="package-container">
					<div class="cont" id="package-header-container">
						<div class="package-shadow" id="package-header">
							<div class="cont" id="package-icon">
								<img
									id="package-icon-image"
									src={
										meta()?.icon == undefined
											? "/icons/default_instance.png"
											: meta()!.icon
									}
									onerror={(e) =>
										((e.target as any).src = "/icons/default_instance.png")
									}
								/>
							</div>
							<div class="col" id="package-details">
								<div class="cont" id="package-upper-details">
									<div id="package-name">{meta()!.name}</div>
									{/* <div id="package-id">{packageId}</div> */}
									<Show when={repoInfo() != undefined}>
										<div
											id="package-repo"
											style={`background-color:${repoInfo()!.meta.color == undefined
												? "var(--fg2)"
												: repoInfo()!.meta.color
												};color:${repoInfo()!.meta.text_color == undefined
													? "var(--bg)"
													: repoInfo()!.meta.text_color
												}`}
										>
											{beautifyString(repoInfo()!.id).toLocaleUpperCase()}
										</div>
									</Show>
									<Show when={properties()!.types != undefined}>
										<PackageLabels
											categories={[]}
											loaders={[]}
											packageTypes={canonicalizeListOrSingle(
												properties()!.types
											)}
										/>
									</Show>
									<Show when={meta()!.downloads != undefined}>
										<div class="cont bold" style="color: var(--fg3);gap:0.2rem">
											<Icon icon={Download} size="1rem" />
											{formatNumber(meta()!.downloads!)}
										</div>
									</Show>
								</div>
								<div class="cont" id="package-short-description">
									{shortDescription()}
								</div>
								<PackageLabels
									categories={
										meta()!.categories == undefined ? [] : meta()!.categories!
									}
									loaders={
										properties()!.supported_loaders == undefined
											? []
											: properties()!.supported_loaders!
									}
									packageTypes={[]}
								/>
							</div>
						</div>
					</div>
					<div id="package-contents">
						<div id="package-body">
							<div class="package-shadow" id="package-tabs">
								<div
									class={`cont package-tab ${selectedTab() == "description" ? "selected" : ""
										}`}
									onclick={() => setSelectedTab("description")}
								>
									<Icon icon={Text} size="1rem" />
									Description
								</div>
								<div
									class={`cont package-tab ${selectedTab() == "versions" ? "selected" : ""
										}`}
									onclick={() => setSelectedTab("versions")}
								>
									<Icon icon={Folder} size="1rem" />
									Versions
								</div>
								<div
									class={`cont package-tab ${selectedTab() == "gallery" ? "selected" : ""
										}`}
									onclick={() => setSelectedTab("gallery")}
								>
									<Icon icon={Picture} size="1rem" />
									Gallery
								</div>
							</div>
							<div class="cont col package-shadow" id="package-tab-contents">
								<Show when={selectedTab() == "description"}>
									<div
										class="cont col package-description"
										innerHTML={longDescription()}
									></div>
								</Show>
								<Show when={selectedTab() == "versions"}>
									<div class="cont fullwidth">
										<PackageVersions
											packageId={packageId}
											props={properties()!}
											onInstall={(version) => {
												setInstallVersion(version);
												setShowInstallModal(true);
											}}
											defaultFilters={filters()}
										/>
									</div>
								</Show>
								<Show
									when={
										selectedTab() == "gallery" && meta()!.gallery != undefined
									}
								>
									<div class="cont">
										<PackageGallery gallery={meta()!.gallery!} />
									</div>
								</Show>
							</div>
						</div>
						<div class="package-shadow cont col" id="package-properties">
							<Show when={meta()!.support_link != undefined}>
								<Property icon={Heart} label="Donate" color="var(--error)">
									<OpenButton url={meta()!.support_link} />
								</Property>
							</Show>
							<Show when={meta()!.website != undefined}>
								<Property icon={Globe} label="Website">
									<OpenButton url={meta()!.website} />
								</Property>
							</Show>
							<Show when={meta()!.documentation != undefined}>
								<Property icon={Book} label="Documentation">
									<OpenButton url={meta()!.documentation} />
								</Property>
							</Show>
							<Show when={meta()!.community != undefined}>
								<Property icon={User} label="Community">
									<OpenButton url={meta()!.community} />
								</Property>
							</Show>
							<Show when={meta()!.source != undefined}>
								<Property icon={CurlyBraces} label="Source">
									<OpenButton url={meta()!.source} />
								</Property>
							</Show>
							<Show when={meta()!.issues != undefined}>
								<Property icon={Warning} label="Issue Tracker">
									<OpenButton url={meta()!.issues} />
								</Property>
							</Show>
							<For each={canonicalizeListOrSingle(meta()!.authors)}>
								{(author) => (
									<Property icon={User} label="Author">
										{author}
									</Property>
								)}
							</For>
							<Property icon={Key} label="License">
								{meta()!.license == undefined ? (
									"Unknown"
								) : meta()!.license!.startsWith("http") ? (
									<a href={meta()!.license} target="_blank">
										Open
									</a>
								) : meta()!.license!.length > 17 ? (
									`${meta()!.license!.slice(0, 17)}...`
								) : (
									meta()!.license
								)}
							</Property>
							<Property icon={Hashtag} label="ID">
								{parsePkgRequest(packageId).id}
							</Property>
						</div>
					</div>
				</div>
				<br />
				<br />
				<br />
			</div>
			<PackageInstallModal
				packageId={parsePkgRequest(packageId).id}
				packageRepo={parsePkgRequest(packageId).repository}
				selectedVersion={installVersion()}
				visible={showInstallModal()}
				onClose={() => setShowInstallModal(false)}
				onShowVersions={() => setSelectedTab("versions")}
			/>
		</Show>
	);
}

export interface ViewPackageProps {
	setFooterData: (data: FooterData) => void;
}

function Property(props: PropertyProps) {
	let color = props.color == undefined ? "var(--fg)" : props.color;

	return (
		<div class="package-property">
			<div class="cont package-property-icon" style={`color:${color}`}>
				<Icon icon={props.icon} size="1rem" />
			</div>
			<div class="cont package-property-label">{props.label}</div>
			<div class="cont package-property-value">{props.children}</div>
		</div>
	);
}

function OpenButton({ url }: { url: string | undefined }) {
	return <IconButton
		icon={Popout}
		size="1.5rem"
		color="var(--bg2)"
		hoverBackground="var(--bg3)"
		onClick={async () => {
			if (url != undefined) {
				try {
					await open(url);
				} catch (e) {
					console.error(e);
					new WebviewWindow("external", { url: url, title: "External Site" });
				}
			}
		}}
	/>;
}

interface PropertyProps {
	icon: (props: HasWidthHeight) => JSX.Element;
	label: string;
	children: JSX.Element;
	color?: string;
}
