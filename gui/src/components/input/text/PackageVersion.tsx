import { createEffect, createSignal, Match, Switch } from "solid-js";
import Tip from "../../dialog/Tip";
import Icon from "../../Icon";

import "./PackageVersion.css";
import { Lock } from "../../../icons";
import Dropdown from "../select/Dropdown";

// An editable / lockable package version
export default function PackageVersion(props: PackageVersionProps) {
	let isEditable = props.onEdit != undefined;
	let [newVersion, setNewVersion] = createSignal<string | undefined>();

	let inputElement!: HTMLInputElement;

	createEffect(() => {
		if (newVersion() != undefined) {
			if (props.onStartEdit != undefined) {
				props.onStartEdit!();
			}

			if (inputElement != undefined) {
				inputElement.focus();
			}
		}
	});

	return (
		<Switch>
			<Match when={newVersion() != undefined}>
				<Switch>
					<Match when={props.versionOptions == undefined}>
						<form
							onsubmit={(e) => {
								e.preventDefault();

								if (newVersion()!.length == 0) {
									props.onEdit!(undefined);
								} else {
									props.onEdit!(newVersion());
								}
								setNewVersion(undefined);
							}}
						>
							<input
								class="cont package-content-version package-content-version-edit"
								value={newVersion()}
								onchange={(e) => setNewVersion(e.target.value)}
								onkeydown={(e: any) => {
									// Unfocus on escape
									if (e.keyCode == 27) {
										e.target.blur();
										setNewVersion(undefined);
									}
								}}
								onfocusout={() => setNewVersion(undefined)}
								ref={inputElement}
							/>
						</form>
					</Match>
					<Match when={props.versionOptions != undefined}>
						<div class="cont" style="width:8rem">
							<Dropdown
								options={props.versionOptions!.map((x) => {
									return {
										value: x,
										contents: x,
									};
								})}
								startOpen
								selected={props.installedVersion}
								onChange={(version) => {
									props.onEdit!(version);
									setNewVersion(undefined);
								}}
								zIndex="5"
								onClose={() => setNewVersion(undefined)}
							/>
						</div>
					</Match>
				</Switch>
			</Match>
			<Match when={props.configuredVersion != undefined}>
				<Tip
					tip={
						isEditable
							? `Version locked at ${props.configuredVersion}. Click to edit.`
							: `Version locked at ${props.configuredVersion}`
					}
					side="top"
				>
					<div
						class="cont start bubble-hover package-content-version"
						onclick={() => {
							if (isEditable) {
								setNewVersion(props.configuredVersion);
							}
						}}
					>
						<Icon icon={Lock} size="1rem" />
						{props.configuredVersion}
					</div>
				</Tip>
			</Match>
			<Match when={props.installedVersion != undefined}>
				<div
					class="cont bubble-hover package-content-version"
					onclick={() => {
						if (isEditable) {
							setNewVersion(props.installedVersion);
						}
					}}
				>
					{props.installedVersion}
				</div>
			</Match>
			<Match when={true}>
				<div
					class="cont bubble-hover package-content-version"
					onclick={() => {
						if (isEditable) {
							setNewVersion("");
						}
					}}
				>
					Any Version{isEditable ? " - Click to edit" : ""}
				</div>
			</Match>
		</Switch>
	);
}

export interface PackageVersionProps {
	installedVersion?: string;
	configuredVersion?: string;
	onEdit?: (version: string | undefined) => void;
	onStartEdit?: () => void;
	versionOptions?: string[];
}
