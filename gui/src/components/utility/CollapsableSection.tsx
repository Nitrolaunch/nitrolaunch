import { createSignal, JSX, Match, Switch } from "solid-js";
import Icon from "../Icon";
import { AngleDown, AngleRight } from "../../icons";
import "./CollapsableSection.css";
import DisplayShow from "./DisplayShow";

export default function CollapsableSection(props: CollapsableSectionProps) {
	let [isOpen, setIsOpen] = createSignal(false);

	return (
		<div class="cont col fullwidth collapsable-section">
			<div
				class={`split fullwidth collapsable-section-header ${isOpen() ? "open" : ""}`}
				onclick={() => setIsOpen(!isOpen())}
			>
				<div class="cont start collapsable-section-title">{props.title}</div>
				<div class="cont end">
					<Switch>
						<Match when={!isOpen()}>
							<Icon icon={AngleRight} size="1rem" />
						</Match>
						<Match when={isOpen()}>
							<Icon icon={AngleDown} size="1rem" />
						</Match>
					</Switch>
				</div>
			</div>
			<DisplayShow when={isOpen()} style="width:100%">
				<div class="cont fullwidth collapsable-section-contents">
					{props.children}
				</div>
			</DisplayShow>
		</div>
	);
}

export interface CollapsableSectionProps {
	title: string;
	children: JSX.Element;
}
