import { createMemo, createResource, Show } from "solid-js";
import Modal from "../../components/dialog/Modal";
import { ArrowLeft, Box, Delete, Dumbbell, Popout, Trash } from "../../icons";
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
import Icon from "../../components/Icon";
import InlineSelect from "../../components/input/select/InlineSelect";
import { canonicalizeListOrSingle } from "../../utils/values";

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

	let selectedOverrides = createMemo(() => {
		let out: string[] = [];
		if (props.props == undefined) {
			return out;
		}
		if (props.props!.suppressed()) {
			out.push("suppressed");
		}
		if (props.props!.forced()) {
			out.push("forced");
		}
		return out;
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
			</div>
			<div class="cont start fullwidth" id="configured-package-modal-tags">
				<Show when={currentRepoInfo() != undefined}>
					<div
						id="package-repo"
						style={`background-color:${currentRepoInfo()!.meta.color == undefined
							? "var(--fg2)"
							: currentRepoInfo()!.meta.color
							};color:${currentRepoInfo()!.meta.text_color == undefined
								? "var(--bg)"
								: currentRepoInfo()!.meta.text_color
							}`}
					>
						{beautifyString(currentRepoInfo()!.id).toLocaleUpperCase()}
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
				<Show when={props.props!.pkg.isDerived}>
					<div class="cont configured-package-derive-indicator">
						DERIVED
					</div>
				</Show>
				<Show when={props.props!.suppressed()}>
					<div class="cont tag" style="color:var(--warning);border-color:var(--warning);background-color:var(--packagebg);font-size:0.8rem;height:1.5rem">
						<Icon icon={Delete} size="0.75rem" />
						SUPPRESSED
					</div>
				</Show>
				<Show when={props.props!.forced()}>
					<div class="cont tag" style="color:var(--error);border-color:var(--error);background-color:var(--errorbg);font-size:0.8rem;height:1.5rem">
						<Icon icon={Dumbbell} size="0.75rem" />
						FORCED
					</div>
				</Show>
			</div>
			<div class="cont fullwidth" id="configured-package-modal-description">
				{shortDescription()}
			</div>
			<div></div>
			<div></div>
			<div class="cont bold">Overrides</div>
			<div class="cont col start fullwidth" style="align-items:flex-start">
				<InlineSelect
					options={[
						{
							value: "suppressed",
							contents: "Suppress",
							color: "var(--warning)",
							selectedBgColor: "var(--packagebg)"
						},
						{
							value: "forced",
							contents: "Force",
							color: "var(--error)",
							selectedBgColor: "var(--errorbg)",
						}
					]}
					selected={selectedOverrides()}
					onChangeMulti={(values) => {
						props.props!.setOverrides((overrides) => {
							let suppressed = canonicalizeListOrSingle(overrides.suppress);
							let forced = canonicalizeListOrSingle(overrides.force);
							let pkg = props.props!.pkg.pkg;

							if (values!.includes("suppressed")) {
								if (!suppressed.includes(pkg)) {
									suppressed.push(pkg);
								}
							} else {
								suppressed = suppressed.filter((x) => x != pkg);
							}

							if (values!.includes("forced")) {
								if (!forced.includes(pkg)) {
									forced.push(pkg);
								}
							} else {
								forced = forced.filter((x) => x != pkg);
							}

							return { suppress: suppressed, force: forced };
						});
						props.props!.setDirty();
					}}
					columns={2}
					connected={false}
					checkboxes
				/>
			</div>
		</Modal>
	);
}

export interface ConfiguredPackageModalProps {
	props?: ConfiguredPackageProps;
	onClose: () => void;
}
