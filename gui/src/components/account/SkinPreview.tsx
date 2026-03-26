import { CrouchAnimation, IdleAnimation, RunningAnimation, SkinViewer, WalkingAnimation } from "skinview3d";
import { createEffect, createSignal, onMount, Show } from "solid-js";
import "./SkinPreview.css";
import Icon from "../Icon";
import { AngleDown, ArrowRight, Feather, Speed } from "../../icons";

export default function SkinPreview(props: SkinPreviewProps) {
	let canvas!: HTMLCanvasElement;

	let [viewer, setViewer] = createSignal<SkinViewer | undefined>();
	onMount(() => {
		setViewer(new SkinViewer({
			canvas: canvas,
			width: canvas.getBoundingClientRect().width,
			height: canvas.getBoundingClientRect().height,
			pixelRatio: props.light == true ? 0.5 : "match-device",
			enableControls: props.light != true,
			// Zoom in capes
			zoom: props.light == true && props.skin == undefined ? 1.5 : undefined,
		}));
	});

	let [elytra, setElytra] = createSignal(false);
	let [animation, setAnimation] = createSignal<string | undefined>();

	function toggleAnimation(anim: string) {
		if (animation() == anim) {
			setAnimation(undefined);
		} else {
			setAnimation(anim);
		}
	}

	createEffect(() => {
		props.skin;
		if (viewer() == undefined) {
			return;
		}
		if (props.skin == undefined) {
			viewer()!.resetSkin();
		} else {
			viewer()!.loadSkin(props.skin);
		}
	});

	createEffect(() => {
		props.cape;
		if (viewer() == undefined) {
			return;
		}
		if (props.cape == undefined) {
			viewer()!.resetCape();
		} else {
			viewer()!.loadCape(props.cape, { backEquipment: elytra() ? "elytra" : "cape" });
		}
	});

	createEffect(() => {
		animation();
		if (viewer() == undefined) {
			return;
		}
		if (animation() == "walking") {
			viewer()!.animation = new WalkingAnimation();
		} else if (animation() == "running") {
			viewer()!.animation = new RunningAnimation();
		} else if (animation() == "crouch") {
			viewer()!.animation = new CrouchAnimation();
		} else {
			viewer()!.animation = props.light == true ? null : new IdleAnimation();
		}
	});

	let canvasHeight = () => props.showControls ? "90%" : "100%";

	return <div class="skin-preview">
		<canvas ref={canvas} style={`width:100%;height:${canvasHeight()};margin-bottom:0`}></canvas>
		<Show when={props.showControls}>
			<div class="skin-preview-controls">
				<div
					class={`cont skin-preview-control ${animation() == "walking" ? "active" : ""}`}
					onclick={() => toggleAnimation("walking")}
				>
					<Icon icon={ArrowRight} size="1rem" />
				</div>
				<div
					class={`cont skin-preview-control ${animation() == "running" ? "active" : ""}`}
					onclick={() => toggleAnimation("running")}
				>
					<Icon icon={Speed} size="1rem" />
				</div>
				<div
					class={`cont skin-preview-control ${animation() == "crouch" ? "active" : ""}`}
					onclick={() => toggleAnimation("crouch")}
				>
					<Icon icon={AngleDown} size="1rem" />
				</div>
				<div
					class={`cont skin-preview-control ${elytra() ? "active" : ""}`}
					onclick={() => setElytra(!elytra())}
				>
					<Icon icon={Feather} size="1rem" />
				</div>
			</div>
		</Show>
	</div>
}

export interface SkinPreviewProps {
	skin?: string;
	cape?: string;
	showControls: boolean;
	light?: boolean;
}
