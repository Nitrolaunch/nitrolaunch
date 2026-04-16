import { onMount } from "solid-js";
import "./Tips.css";

export default function Tips() {
	let tip!: HTMLDivElement;

	onMount(() => {
		document.addEventListener("mousemove", (e) => {
			if (tip == undefined) {
				return;
			}
			let elem = document.elementFromPoint(
				e.clientX,
				e.clientY,
			) as HTMLElement | null;
			// Walk tree to find elem
			while (elem != null) {
				if (elem.id == "root") {
					elem = null;
					break;
				}

				if (elem!.dataset.tip != undefined) {
					break;
				}

				elem = elem!.parentElement;
			}

			if (elem == null) {
				tip.style.display = "none";
				return;
			}
			let tipText = elem.dataset.tip!;

			tip.style.display = "";
			tip.innerText = tipText;
			tip.style.top = `${e.clientY}px`;
			tip.style.left = `${e.clientX}px`;
		});
	});

	return <div class="cont" id="tip" ref={tip} style="display:none"></div>;
}
