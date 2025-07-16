import { createMemo, For, Show } from "solid-js";
import {
	getAllLoaders,
	getLoaderDisplayName,
	getLoaderImage,
	getPackageTypeColor,
	getPackageTypeDisplayName,
	getPackageTypeIcon,
	PackageCategory,
	packageCategoryDisplayName,
	packageCategoryIcon,
	PackageType,
} from "../../package";
import Icon from "../Icon";
import "./PackageLabels.css";

export default function PackageLabels(props: PackageLabelsProps) {
	let small = props.small == undefined ? false : props.small;

	let allLoaders = createMemo(() => getAllLoaders(props.loaders));

	return (
		<div class={`cont package-labels ${small ? "small" : ""}`}>
			<For each={/* @once */ props.packageTypes}>
				{(type, i) => {
					if (props.limit != undefined && i() >= props.limit) {
						return undefined;
					} else {
						return (
							<div class={`cont package-type ${small ? "small" : ""}`}>
								<div
									class="cont package-type-icon"
									style={`color:${getPackageTypeColor(type)}`}
								>
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
						i() + props.packageTypes.length + props.categories.length >=
							props.limit
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
	packageTypes: PackageType[];
	// The maximum number of labels to include
	limit?: number;
	small?: boolean;
	reverse?: boolean;
}
