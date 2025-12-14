import { For, JSX } from "solid-js";
import Icon, { HasWidthHeight } from "../../Icon";
import "./FloatingTabs.css";

export default function FloatingTabs(props: FloatingTabsProps) {
	return <div
		class="shadow floating-tabs"
		style={`grid-template-columns:repeat(${props.tabs.length}, 1fr)`}
	>
		<For each={props.tabs}>
			{(tab, i) => {
				let isSelected = () => props.selectedTab == tab.id;
				let color = () => isSelected() ? `color:${tab.color}` : "";
				let bgColor = () => isSelected() ? `background-color:${tab.bgColor}` : "";

				let orderClass = i() == 0 ? "first" : i() == props.tabs.length - 1 ? "last" : "";

				return <div
					class={`cont floating-tab ${isSelected() ? "selected" : ""} ${orderClass}`}
					style={`outline-color:${tab.color};${color()};${bgColor()}`}
					onclick={() => props.setTab(tab.id)}
				>
					<Icon icon={tab.icon} size="1rem" />
					{tab.title}
				</div>;
			}}
		</For>
	</div>;
}

export interface FloatingTabsProps {
	tabs: Tab[],
	selectedTab?: string;
	setTab: (tab: string) => void;
}

export interface Tab {
	id: string;
	title: string;
	icon: (props: HasWidthHeight) => JSX.Element;
	color: string;
	bgColor: string;
}
