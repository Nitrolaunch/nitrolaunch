import { createSignal, For, Show } from "solid-js";
import Icon from "../Icon";
import { AngleLeft, AngleRight } from "../../icons";
import Modal from "../dialog/Modal";

export default function PackageGallery(props: PackageGalleryProps) {
	// The URL and index of the previewed gallery entry. Undefined if not shown.
	let [preview, setPreview] = createSignal<
		[string, number] | undefined
	>();

	return <div id="package-gallery">
		<For each={props.gallery}>
			{(entry, i) => (
				<img
					class="package-gallery-entry input-shadow bubble-hover"
					src={entry}
					onclick={() => setPreview([entry, i()])}
				/>
			)}
		</For>
		<Modal
			width="55rem"
			visible={preview() != undefined}
			onClose={() => setPreview(undefined)}
		>
			<img
				id="package-gallery-preview"
				src={preview()![0]}
				onclick={() => setPreview(undefined)}
			/>
			<Show when={preview()![1] > 0}>
				<div
					class="cont bubble-hover pop-in-fast package-gallery-arrow"
					style="left:1rem"
					onclick={() => {
						let i = preview()![1];
						setPreview([props.gallery[i - 1], i - 1]);
					}}
				>
					<Icon icon={AngleLeft} size="1.5rem" />
				</div>
			</Show>
			<Show when={preview()![1] < props.gallery.length - 1}>
				<div
					class="cont bubble-hover pop-in-fast package-gallery-arrow"
					style="right:1rem"
					onclick={() => {
						let i = preview()![1];
						setPreview([props.gallery[i + 1], i + 1]);
					}}
				>
					<Icon icon={AngleRight} size="1.5rem" />
				</div>
			</Show>
		</Modal>
	</div>;
}

export interface PackageGalleryProps {
	gallery: string[];
}
