import { createSignal, For } from "solid-js";
import "./EditableList.css";
import IconButton from "./IconButton";
import { Plus, Trash } from "../../icons";

export default function EditableList(props: EditableListProps) {
	let reorderable = props.reorderable == undefined ? true : props.reorderable;

	let [newItem, setNewItem] = createSignal("");

	let addNewItem = () => {
		if (newItem().length == 0) {
			return;
		}

		let items = props.items.concat();
		items.push(newItem());
		props.setItems(items);
		setNewItem("");
	};

	return (
		<div class="cont col start fullwidth editable-list">
			<For each={props.items}>
				{(item, index) => {
					let [isDragHovered, setIsDragHovered] = createSignal(false);

					return (
						<div
							class="editable-list-item"
							draggable={reorderable ? "true" : "false"}
							ondragstart={(e) => {
								e.dataTransfer!.setData(
									"Text",
									JSON.stringify({
										index: index(),
										itemValue: item,
									} as DragData)
								);
							}}
							ondragover={(e) => e.preventDefault()}
							ondrop={(e) => {
								let data = JSON.parse(
									e.dataTransfer!.getData("Text")
								) as DragData;
								if (data.index != index()) {
									let items = props.items.concat();
									items.splice(data.index, 1);
									let newIndex = data.index < index() ? index() : index() + 1;
									items.splice(newIndex, 0, data.itemValue);
									props.setItems(items);
									setIsDragHovered(false);
								}
							}}
							ondragenter={() => setIsDragHovered(true)}
							ondragleave={() => setIsDragHovered(false)}
						>
							<div
								class={`cont start fullheight editable-list-item-contents ${
									isDragHovered() ? "drag-hovered" : ""
								}`}
							>
								{item}
							</div>
							<div class="cont fullheight">
								<IconButton
									icon={Trash}
									size="1.5rem"
									color="var(--bg2)"
									selectedColor=""
									border="var(--bg3)"
									hoverBorder="var(--bg4)"
									selected={false}
									onClick={() => {
										let items = props.items.concat();
										items.splice(index(), 1);
										props.setItems(items);
									}}
								/>
							</div>
						</div>
					);
				}}
			</For>
			<div class="editable-list-item">
				<div class="cont start fullheight">
					<form
						class="fullwidth"
						onsubmit={(e) => {
							e.preventDefault();
							e.stopPropagation();
							addNewItem();
						}}
					>
						<input
							type="text"
							value={newItem()}
							onkeyup={(e: any) => setNewItem(e.target.value)}
						/>
					</form>
				</div>
				<div class="cont fullheight">
					<IconButton
						icon={Plus}
						size="1.5rem"
						color="var(--bg2)"
						selectedColor=""
						border="var(--bg3)"
						hoverBorder="var(--bg4)"
						selected={false}
						onClick={addNewItem}
					/>
				</div>
			</div>
		</div>
	);
}

interface DragData {
	index: number;
	itemValue: string;
}

export interface EditableListProps {
	items: string[];
	setItems: (value: string[]) => void;
	reorderable?: boolean;
}
