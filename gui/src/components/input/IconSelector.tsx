import { createResource, For } from "solid-js";
import "./IconSelector.css";

export default function IconSelector(props: IconSelectorProps) {
	let [availableIcons, _] = createResource(async () => {
		return [
			"/icons/default_instance.png",
			"/icons/fabric.png",
			"/icons/quilt.png",
			"/icons/paper.png",
		];
	});

	let selectedIcon = () =>
		props.icon == undefined ? "/icons/default_instance.png" : props.icon;

	return (
		<div class="fullwidth">
			<div class="fullwidth icon-selector">
				<SelectableIcon icon={selectedIcon()} onSelect={() => {}} isSelected />
				<For each={availableIcons()}>
					{(icon) => (
						<SelectableIcon
							icon={icon}
							onSelect={() => {
								if (icon == "/icons/default_instance.png") {
									props.setIcon(undefined);
								} else {
									props.setIcon(icon);
								}
							}}
							isSelected={false}
						/>
					)}
				</For>
			</div>
		</div>
	);
}

function SelectableIcon(props: SelectableIconProps) {
	return (
		<div
			class={`cont icon-selector-icon ${props.isSelected ? "selected" : ""}`}
			onclick={props.onSelect}
		>
			<img src={props.icon} class="icon-selector-icon-image" />
		</div>
	);
}

interface SelectableIconProps {
	icon: string;
	onSelect: () => void;
	isSelected: boolean;
}

export interface IconSelectorProps {
	icon: string | undefined;
	setIcon: (value: string | undefined) => void;
}
