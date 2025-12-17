import { createResource, createSignal, For, Show } from "solid-js";
import "./IconSelector.css";
import { invoke } from "@tauri-apps/api/core";
import { getInstanceIconSrc } from "../../../utils";
import Icon from "../../Icon";
import { Folder } from "../../../icons";
import { open } from "@tauri-apps/plugin-dialog";
import { errorToast } from "../../dialog/Toasts";

export default function IconSelector(props: IconSelectorProps) {
	let selectedIcon = () =>
		props.icon == undefined
			? "builtin:/icons/default_instance.png"
			: props.icon;

	let selectedOrDerivedIcon = () =>
		props.icon == undefined
			? props.derivedIcon == undefined
				? "builtin:/icons/default_instance.png"
				: props.derivedIcon
			: props.icon;

	let [isOpen, setIsOpen] = createSignal(false);

	let [availableIcons, iconMethods] = createResource(
		() => (props.derivedIcon == undefined ? "" : props.derivedIcon),
		async () => {
			let availableIcons: string[];
			try {
				availableIcons = await invoke("get_available_icons");
			} catch (e) {
				console.error(e);
				availableIcons = [];
			}

			let defaultIcons = [
				"builtin:/icons/default_instance.png",
				"builtin:/icons/minecraft.png",
				"builtin:/icons/fabric.png",
				"builtin:/icons/quilt.png",
				"builtin:/icons/paper.png",
				"builtin:/icons/folia.png",
				"builtin:/icons/forge.png",
				"builtin:/icons/neoforge.png",
				"builtin:/icons/sponge.png",
			];

			let out = defaultIcons;
			out = out.concat(availableIcons);

			// Just in case it gets removed add the currently selected icon and the derived icon
			if (props.icon != undefined && !out.includes(props.icon)) {
				out.push(props.icon);
			}
			if (props.derivedIcon != undefined && !out.includes(props.derivedIcon)) {
				out.push(props.derivedIcon);
			}

			return out;
		}
	);

	async function addIcon() {
		try {
			let file = (await open({
				directory: false,
				title: "Select Icon",
				filters: [
					{
						name: "Image",
						extensions: ["png", "jpeg", "gif", "ico", "webp", "tiff", "svg"],
					},
				],
				multiple: false,
			})) as string;

			props.setIcon(file);

			await invoke("save_icon", { icon: file });
			iconMethods.refetch();
		} catch (e) {
			errorToast("Failed to select icon: " + e);
		}
	}

	return (
		<div class="icon-selector">
			<div class="cont" style="width: 10rem">
				<SelectableIcon
					icon={selectedOrDerivedIcon()}
					onSelect={() => setIsOpen(!isOpen())}
					isSelected={false}
					isDerived={props.derivedIcon != undefined && props.icon == undefined}
				/>
			</div>
			<Show when={isOpen()}>
				<div
					class="cont col start shadow icon-selector-panel"
					onmouseleave={() => setIsOpen(false)}
				>
					<div class="fullwidth icon-selector-grid">
						<For each={availableIcons()}>
							{(icon) => (
								<SelectableIcon
									icon={icon}
									onSelect={() => {
										if (icon == "builtin:/icons/default_instance.png") {
											props.setIcon(undefined);
										} else {
											props.setIcon(icon);
										}
									}}
									isSelected={icon == selectedIcon()}
									isDerived={
										icon == props.derivedIcon && props.icon == undefined
									}
								/>
							)}
						</For>
						<div
							class={`cont bubble-hover shadow icon-selector-icon`}
							onclick={addIcon}
						>
							<Icon icon={Folder} size="2rem" />
						</div>
					</div>
				</div>
			</Show>
		</div>
	);
}

function SelectableIcon(props: SelectableIconProps) {
	let src = () => getInstanceIconSrc(props.icon);

	return (
		<div
			class={`cont bubble-hover shadow icon-selector-icon ${
				props.isSelected ? "selected" : ""
			} ${props.isDerived ? "derived" : ""}`}
			onclick={props.onSelect}
		>
			<img src={src()} class="icon-selector-icon-image" />
		</div>
	);
}

interface SelectableIconProps {
	icon: string;
	onSelect: () => void;
	isSelected: boolean;
	isDerived: boolean;
}

export interface IconSelectorProps {
	icon: string | undefined;
	setIcon: (value: string | undefined) => void;
	derivedIcon: string | undefined;
}
