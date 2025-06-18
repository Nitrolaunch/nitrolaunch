import { useParams } from "@solidjs/router";
import "./ViewPackage.css";
import { invoke } from "@tauri-apps/api";
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
import { FooterMode } from "../../components/launch/Footer";
import Icon, { HasWidthHeight } from "../../components/Icon";
import {
	AngleLeft,
	AngleRight,
	Book,
	CurlyBraces,
	Folder,
	Globe,
	Hashtag,
	Heart,
	Key,
	Picture,
	Text,
	User,
	Warning,
} from "../../icons";
import Modal from "../../components/dialog/Modal";
import PackageLabels from "../../components/package/PackageLabels";
import { RepoInfo } from "../../package";
import { beautifyString, parsePkgRequest } from "../../utils";
import PackageVersions from "../../components/package/PackageVersions";
import PackageInstallModal from "../../components/package/PackageInstallModal";
import { canonicalizeListOrSingle } from "../../utils/values";

export default function ViewPackage(props: ViewPackageProps) {
	let params = useParams();

	let packageId = params.id;

	let [meta] = createResource(updateMeta);
	let [properties] = createResource(updateProps);

	let [repoInfo, setRepoInfo] = createSignal<RepoInfo | undefined>(undefined);
	let [shortDescription, setShortDescription] = createSignal("");
	let [longDescription, setLongDescription] = createSignal("");

	let [selectedTab, setSelectedTab] = createSignal("description");
	let [galleryPreview, setGalleryPreview] = createSignal<
		[string, number] | undefined
	>();

	let [showInstallModal, setShowInstallModal] = createSignal(false);
	let [installVersion, setInstallVersion] = createSignal<string | undefined>();

	createEffect(() => {
		props.setFooterData({
			selectedItem: "",
			mode: FooterMode.InstallPackage,
			action: () => setShowInstallModal(true),
		});
	});

	async function updateMeta() {
		let [meta, repos] = (await Promise.all([
			invoke("get_package_meta", {
				package: packageId,
			}),
			invoke("get_package_repos"),
		])) as [PackageMeta, RepoInfo[]];

		let request = parsePkgRequest(packageId);
		if (request.repo != undefined)
			for (let repo of repos) {
				if (repo.id == request.repo) {
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

		return meta;
	}

	async function updateProps() {
		try {
			let props: PackageProperties = await invoke("get_package_props", {
				package: packageId,
			});

			return props;
		} catch (e) {
			errorToast("Failed to load package: " + e);
		}
	}

	return (
		<Show when={meta() != undefined && properties() != undefined}>
			<div class="cont col" style="width:100%">
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
											style={`background-color:${
												repoInfo()!.meta.color == undefined
													? "var(--fg2)"
													: repoInfo()!.meta.color
											};color:${
												repoInfo()!.meta.text_color == undefined
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
					<div id="package-contents">
						<div id="package-body">
							<div class="package-shadow" id="package-tabs">
								<div
									class={`cont package-tab ${
										selectedTab() == "description" ? "selected" : ""
									}`}
									onclick={() => setSelectedTab("description")}
								>
									<Icon icon={Text} size="1rem" />
									Description
								</div>
								<div
									class={`cont package-tab ${
										selectedTab() == "versions" ? "selected" : ""
									}`}
									onclick={() => setSelectedTab("versions")}
								>
									<Icon icon={Folder} size="1rem" />
									Versions
								</div>
								<div
									class={`cont package-tab ${
										selectedTab() == "gallery" ? "selected" : ""
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
										class="cont col"
										id="package-description"
										innerHTML={longDescription()}
									></div>
								</Show>
								<Show when={selectedTab() == "versions"}>
									<div class="cont">
										<PackageVersions
											packageId={packageId}
											props={properties()!}
											backgroundColor="var(--bg)"
											onInstall={(version) => {
												setInstallVersion(version);
												setShowInstallModal(true);
											}}
										/>
									</div>
								</Show>
								<Show
									when={
										selectedTab() == "gallery" && meta()!.gallery != undefined
									}
								>
									<div class="cont">
										<div id="package-gallery">
											<For each={meta()!.gallery!}>
												{(entry, i) => (
													<img
														class="package-gallery-entry"
														src={entry}
														onclick={() => setGalleryPreview([entry, i()])}
													/>
												)}
											</For>
										</div>
									</div>
									<Modal
										width="55rem"
										visible={galleryPreview() != undefined}
										onClose={() => setGalleryPreview(undefined)}
									>
										<img
											id="package-gallery-preview"
											src={galleryPreview()![0]}
											onclick={() => setGalleryPreview(undefined)}
										/>
										<div
											class="package-gallery-arrow"
											style="left:1rem"
											onclick={() => {
												if (galleryPreview() != undefined) {
													let i = galleryPreview()![1];
													if (i > 0) {
														setGalleryPreview([meta()!.gallery![i - 1], i - 1]);
													}
												}
											}}
										>
											<Icon icon={AngleLeft} size="2rem" />
										</div>
										<div
											class="package-gallery-arrow"
											style="right:1rem"
											onclick={() => {
												if (galleryPreview() != undefined) {
													let i = galleryPreview()![1];
													if (i < meta()!.gallery!.length - 1) {
														setGalleryPreview([meta()!.gallery![i + 1], i + 1]);
													}
												}
											}}
										>
											<Icon icon={AngleRight} size="2rem" />
										</div>
									</Modal>
								</Show>
							</div>
						</div>
						<div class="package-shadow cont col" id="package-properties">
							<Show when={meta()!.website != undefined}>
								<Property icon={Globe} label="Website">
									<a href={meta()!.website} target="_blank">
										Open
									</a>
								</Property>
							</Show>
							<Show when={meta()!.support_link != undefined}>
								<Property icon={Heart} label="Donate" color="var(--error)">
									<a href={meta()!.support_link} target="_blank">
										Open
									</a>
								</Property>
							</Show>
							<Show when={meta()!.documentation != undefined}>
								<Property icon={Book} label="Documentation">
									<a href={meta()!.documentation} target="_blank">
										Open
									</a>
								</Property>
							</Show>
							<Show when={meta()!.source != undefined}>
								<Property icon={CurlyBraces} label="Source">
									<a href={meta()!.source} target="_blank">
										Open
									</a>
								</Property>
							</Show>
							<Show when={meta()!.issues != undefined}>
								<Property icon={Warning} label="Issue Tracker">
									<a href={meta()!.issues} target="_blank">
										Open
									</a>
								</Property>
							</Show>
							<Show when={meta()!.community != undefined}>
								<Property icon={User} label="Community">
									<a href={meta()!.community} target="_blank">
										Open
									</a>
								</Property>
							</Show>
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
				packageRepo={parsePkgRequest(packageId).repo}
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

interface PropertyProps {
	icon: (props: HasWidthHeight) => JSX.Element;
	label: string;
	children: JSX.Element;
	color?: string;
}
