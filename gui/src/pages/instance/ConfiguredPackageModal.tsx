import { createMemo, createResource, Show } from "solid-js";
import Modal from "../../components/dialog/Modal";
import { ArrowLeft, Box, Popout, Trash } from "../../icons";
import {
	ConfiguredPackageCategory,
	ConfiguredPackageProps,
} from "./PackagesConfig";
import { RepoInfo } from "../../package";
import { invoke } from "@tauri-apps/api/core";
import { beautifyString } from "../../utils";
import "./ConfiguredPackageModal.css";
import PackageVersion from "../../components/input/text/PackageVersion";
import IconTextButton from "../../components/input/button/IconTextButton";
import { useNavigate } from "@solidjs/router";

export default function ConfiguredPackageModal(
	props: ConfiguredPackageModalProps
) {
	let navigate = useNavigate();

	let name = () =>
		props.props == undefined
			? undefined
			: props.props.meta == undefined || props.props.meta.name == undefined
			? props.props.pkg.req.id
			: props.props.meta.name;

	let icon = () =>
		props.props == undefined
			? "/icons/default_instance.png"
			: props.props.meta == undefined || props.props.meta.icon == undefined
			? "/icons/default_instance.png"
			: props.props.meta.icon;

	let shortDescription = () =>
		props.props == undefined
			? undefined
			: props.props.meta == undefined ||
			  props.props.meta.description == undefined
			? undefined
			: props.props.meta.description;

	let [repoInfo, _] = createResource(
		async () => {
			let repos = (await invoke("get_package_repos")) as RepoInfo[];
			let out: { [repo: string]: RepoInfo | undefined } = {};
			for (let repo of repos) {
				out[repo.id] = repo;
			}

			return out;
		},
		{ initialValue: {} }
	);

	let currentRepoInfo = createMemo(() => {
		if (
			props.props == undefined ||
			props.props.pkg.req.repository == undefined
		) {
			return undefined;
		} else {
			return repoInfo()[props.props.pkg.req.repository];
		}
	});

	return (
		<Modal
			visible={props.props != undefined}
			onClose={props.onClose}
			width="35rem"
			padding=".75rem"
			title={name()}
			titleIcon={Box}
			buttons={[
				{
					text: "Back",
					icon: ArrowLeft,
					onClick: props.onClose,
				},
			]}
		>
			<div class="fullwidth" id="configured-package-modal-header">
				<div class="cont fullwidth">
					<img id="configured-package-modal-icon" src={icon()} />
				</div>
				<div class="cont col fullwidth">
					<div
						class="cont end fullwidth"
						id="configured-package-modal-controls"
					>
						<IconTextButton
							icon={Popout}
							text="Go to Page"
							size="1.5rem"
							color="var(--fg3)"
							onClick={() => {
								navigate(`/packages/package/${props.props!.pkg.pkg}`);
							}}
						/>
						<Show
							when={
								!props.props!.pkg.isDerived && props.props!.pkg.isConfigured
							}
						>
							<IconTextButton
								icon={Trash}
								text="Remove"
								size="1.2rem"
								color="var(--error)"
								bgColor="var(--errorbg)"
								onClick={() => {
									let category: ConfiguredPackageCategory = props.props!.pkg
										.isClient
										? "client"
										: props.props!.pkg.isServer
										? "server"
										: "global";

									props.props!.onRemove(props.props!.pkg.pkg, category);
								}}
							/>
						</Show>
					</div>
					<div class="cont end fullwidth" id="configured-package-modal-tags">
						<Show when={currentRepoInfo() != undefined}>
							<div
								id="package-repo"
								style={`background-color:${
									currentRepoInfo()!.meta.color == undefined
										? "var(--fg2)"
										: currentRepoInfo()!.meta.color
								};color:${
									currentRepoInfo()!.meta.text_color == undefined
										? "var(--bg)"
										: currentRepoInfo()!.meta.text_color
								}`}
							>
								{beautifyString(currentRepoInfo()!.id).toLocaleUpperCase()}
							</div>
						</Show>
						<Show when={props.props!.pkg.isDerived}>
							<div class="cont configured-package-derive-indicator">
								DERIVED
							</div>
						</Show>
						<PackageVersion
							configuredVersion={props.props!.pkg.req.version}
							installedVersion={props.props!.pkg.contentVersion}
							onEdit={(version) => {
								props.props!.onVersionChange(version);
								props.onClose();
							}}
						/>
					</div>
				</div>
			</div>
			<div></div>
			<div class="cont fullwidth" id="configured-package-modal-description">
				{shortDescription()}
			</div>
		</Modal>
	);
}

export interface ConfiguredPackageModalProps {
	props?: ConfiguredPackageProps;
	onClose: () => void;
}
