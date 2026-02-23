import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, Box, Delete, Download, Plus, Upload } from "../../icons";
import Modal from "../dialog/Modal";
import "./PackageDiffsPrompt.css";
import { createResource, For, Match, Switch } from "solid-js";
import { PackageMeta } from "../../types";
import Icon from "../Icon";

export default function PackageDiffsPrompt(props: PackageDiffsPromptProps) {
	async function setAnswer(answer: boolean) {
		props.onClose();
		await invoke("answer_yes_no_prompt", { answer: answer });
	}

	let [metas, _] = createResource(() => props.diffs, async (diffs) => {
		let out: { [pkg: string]: PackageMeta | undefined } = {};

		for (let diff of diffs) {
			let pkg = diff.type == "version_changed" ? diff.data[0] : diff.data;
			try {
				let meta: PackageMeta = await invoke("get_package_meta", { package: pkg });
				out[pkg] = meta;
			} catch (e) {
				console.error("Failed to load package: " + e);
			}
		}

		return out;
	}, { initialValue: {} });

	return <Modal
		title="Confirm Packages"
		titleIcon={Box}
		visible={props.diffs != undefined}
		onClose={props.onClose}
		buttons={[
			{
				text: "Cancel",
				icon: Delete,
				onClick: () => setAnswer(false),
			},
			{
				text: "Install",
				icon: Download,
				color: "var(--package)",
				bgColor: "var(--packagebg)",
				onClick: () => setAnswer(true),
			}
		]}
	>
		<For each={props.diffs}>
			{(diff) => {
				let pkg = diff.type == "version_changed" ? diff.data[0] : diff.type.includes("many") ? undefined : diff.data;
				let meta = () => pkg == undefined ? undefined : metas()[pkg];

				let pkgElement = meta() == undefined ? undefined : <div class="cont package-diff-package">
					<img
						class="package-diff-package-icon"
						src={meta() == undefined || meta()!.icon == undefined ? "icons/default_instance.png" : meta()!.icon!}
					/>
					{meta() == undefined || meta()!.name == undefined ? pkg : meta()!.name}
				</div>

				return <div class="cont package-diff">
					<div class={`cont package-diff-indicator ${diff.type}`}>
						<Switch>
							<Match when={diff.type == "added" || diff.type == "many_added"}>
								<Icon icon={Plus} size="1rem" />
							</Match>
							<Match when={diff.type == "removed" || diff.type == "many_removed"}>
								<Icon icon={Delete} size="1rem" />
							</Match>
							<Match when={diff.type == "version_changed"}>
								<Icon icon={Upload} size="1rem" />
							</Match>
						</Switch>
					</div>
					<div class="cont package-diff-info">
						<Switch>
							<Match when={diff.type == "version_changed"}>
								{pkgElement}
								<span class="cont package-diff-version" style="color:var(--error)">{(diff.data as any)[1]}</span>
								<Icon icon={ArrowRight} size="1.25rem" />
								<span class="cont package-diff-version" style="color:var(--instance)">{(diff.data as any)[2]}</span>
							</Match>
							<Match when={diff.type == "many_added" || diff.type == "many_removed"}>
								{diff.data} packages
							</Match>
							<Match when={diff.type != "version_changed" && diff.type != "many_added" && diff.type != "many_removed"}>
								{pkgElement}
							</Match>
						</Switch>
					</div>
				</div>
			}}
		</For>
	</Modal>
}

export interface PackageDiffsPromptProps {
	// Undefined if prompt is not visible
	diffs: PackageDiff[] | undefined;
	onClose: () => void;
}

export type PackageDiff =
	| {
		type: "added",
		data: string
	}
	| {
		type: "many_added",
		data: number
	}
	| {
		type: "removed",
		data: string
	}
	| {
		type: "many_removed",
		data: number
	}
	| {
		type: "version_changed",
		data: [string, string, string]
	};
