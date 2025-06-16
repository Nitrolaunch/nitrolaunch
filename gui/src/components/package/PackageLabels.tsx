import { For, Show } from "solid-js";
import {
	PackageCategory,
	packageCategoryDisplayName,
	packageCategoryIcon,
} from "../../package";
import Icon from "../Icon";
import "./PackageLabels.css";
import { beautifyString } from "../../utils";

export default function PackageLabels(props: PackageLabelsProps) {
	let small = props.small == undefined ? false : props.small;

	let allLoaders = () => {
		let out: Loader[] = [];
		for (let loaderMatch of props.loaders) {
			for (let newLoader of getLoaders(loaderMatch)) {
				if (!out.includes(newLoader)) {
					out.push(newLoader);
				}
			}
		}
		return out;
	};

	return (
		<div class={`cont package-labels ${small ? "small" : ""}`}>
			<For each={props.categories}>
				{(category, i) => {
					if (props.limit != undefined && i() >= props.limit) {
						return undefined;
					} else {
						return (
							<div class={`cont package-category ${small ? "small" : ""}`}>
								<div class="cont package-category-icon">
									<Icon icon={packageCategoryIcon(category)} size="1rem" />
								</div>
								<div class="cont package-category-label">
									{packageCategoryDisplayName(category)}
								</div>
							</div>
						);
					}
				}}
			</For>
			<For each={allLoaders()}>
				{(loader, i) => {
					if (
						props.limit != undefined &&
						i() + props.categories.length >= props.limit
					) {
						return undefined;
					} else {
						return (
							<div class={`cont package-loader ${small ? "small" : ""}`}>
								<div class="cont package-loader-icon">
									<img src={getLoaderImage(loader)} />
								</div>
								<Show when={!small}>
									<div class="cont package-category-label">
										{getLoaderDisplayName(loader)}
									</div>
								</Show>
							</div>
						);
					}
				}}
			</For>
		</div>
	);
}

export interface PackageLabelsProps {
	categories: PackageCategory[];
	loaders: string[];
	// The maximum number of labels to include
	limit?: number;
	small?: boolean;
	reverse?: boolean;
}

export enum Loader {
	Fabric = "fabric",
	Quilt = "quilt",
	Forge = "forge",
	NeoForge = "neoforged",
	Sponge = "sponge",
	SpongeForge = "spongeforge",
}

function getLoaders(modification: string) {
	if (modification == "fabriclike") {
		return [Loader.Fabric, Loader.Quilt];
	} else if (modification == "forgelike") {
		return [Loader.Forge, Loader.NeoForge, Loader.SpongeForge];
	}
	return [modification as Loader];
}

function getLoaderDisplayName(loader: Loader) {
	if (loader == "fabric") {
		return "Fabric";
	} else if (loader == "quilt") {
		return "Quilt";
	} else if (loader == "forge") {
		return "Forge";
	} else if (loader == "neoforged") {
		return "NeoForged";
	} else if (loader == "sponge") {
		return "Sponge";
	} else if (loader == "spongeforge") {
		return "SpongeForge";
	} else {
		return beautifyString(loader);
	}
}

function getLoaderImage(loader: Loader) {
	if (loader == "fabric") {
		return "/icons/fabric.png";
	} else if (loader == "quilt") {
		return "/icons/quilt.png";
	} else if (loader == "forge") {
		return "/icons/forge.png";
	} else if (loader == "neoforged") {
		return "/icons/neoforge.png";
	} else if (loader == "sponge") {
		return "/icons/sponge.png";
	} else if (loader == "spongeforge") {
		return "/icons/sponge.png";
	} else {
		return "/icons/default_instance.png";
	}
}
