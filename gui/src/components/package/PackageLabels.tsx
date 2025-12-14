import { createMemo, createSignal, For, Show } from "solid-js";
import {
	getAllLoaders,
	getLoaderColor,
	getLoaderDisplayName,
	getLoaderImage,
	getPackageTypeColor,
	getPackageTypeDisplayName,
	getPackageTypeIcon,
	Loader,
	PackageCategory,
	packageCategoryDisplayName,
	packageCategoryIcon,
	PackageType,
} from "../../package";
import Icon from "../Icon";
import "./PackageLabels.css";
import { parseVersionedString } from "../../utils";

export default function PackageLabels(props: PackageLabelsProps) {
	let small = props.small == undefined ? false : props.small;
	let tiny = () => props.tiny == undefined ? false : props.tiny;

	let allLoaders = createMemo(() => getAllLoaders(props.loaders));

	let isLimited = () =>
		props.limit != undefined &&
		allLoaders().length + props.packageTypes.length + props.categories.length >
		props.limit;

	return (
		<div class={`cont package-labels ${small ? "small" : ""}`}>
			<For each={/* @once */ props.packageTypes}>
				{(type, i) => {
					if (props.limit != undefined && i() >= props.limit) {
						return undefined;
					} else {
						let color = getPackageTypeColor(type);
						let style = props.tags ? `color:${color};border-color:${color}` : "";
						return (
							<div
								class={`cont package-type ${props.tags ? "tag" : ""} ${small ? "small" : ""}`}
								style={style}
							>
								<div class="cont package-type-icon">
									<Icon icon={getPackageTypeIcon(type)} size="1rem" />
								</div>
								<div class="cont package-type-label">
									{getPackageTypeDisplayName(type)}
								</div>
							</div>
						);
					}
				}}
			</For>
			<For each={/* @once */ props.categories}>
				{(category, i) => {
					if (
						props.limit != undefined &&
						i() + props.packageTypes.length >= props.limit
					) {
						return undefined;
					} else {
						let [isHovered, setIsHovered] = createSignal(false);
						let style = props.tags ? "color:var(--package);border-color:var(--package);background-color:var(--packagebg)" : "";

						return (
							<div
								class={`cont package-category ${props.tags ? "tag" : ""} ${small ? "small" : ""}`}
								style={style}
								onmouseenter={() => setIsHovered(true)}
								onmouseleave={() => setIsHovered(false)}
							>
								<div class="cont package-category-icon">
									<Icon icon={packageCategoryIcon(category)} size="1rem" />
								</div>
								<Show when={!tiny() || isHovered()}>
									<div class="cont package-category-label">
										{packageCategoryDisplayName(category)}
									</div>
								</Show>
							</div>
						);
					}
				}}
			</For>
			<For each={allLoaders()}>
				{(loader, i) => {
					let [loader2, _] = parseVersionedString(loader) as [Loader, string];
					if (
						props.limit != undefined &&
						i() + props.packageTypes.length + props.categories.length >=
						props.limit
					) {
						return undefined;
					} else {
						let [isHovered, setIsHovered] = createSignal(false);
						let color = getLoaderColor(loader);
						let style = props.tags ? `color:${color};border-color:${color}` : "";

						return (
							<div
								class={`cont package-loader ${props.tags ? "tag" : ""} ${small ? "small" : ""}`}
								style={style}
								onmouseenter={() => setIsHovered(true)}
								onmouseleave={() => setIsHovered(false)}
							>
								<div class="cont package-loader-icon">
									<img src={getLoaderImage(loader2)} />
								</div>
								<Show when={(!small && !tiny()) || (tiny() && isHovered())}>
									<div class="cont package-category-label">
										{getLoaderDisplayName(loader2)}
									</div>
								</Show>
							</div>
						);
					}
				}}
			</For>
			<Show when={isLimited()}>...</Show>
		</div>
	);
}

export interface PackageLabelsProps {
	categories: PackageCategory[];
	loaders: string[];
	packageTypes: PackageType[];
	// The maximum number of labels to include
	limit?: number;
	small?: boolean;
	tags?: boolean;
	tiny?: boolean;
	reverse?: boolean;
}
