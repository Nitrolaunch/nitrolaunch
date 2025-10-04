import { createResource, For, Match, Switch } from "solid-js";
import "./InstanceTiles.css";
import { invoke } from "@tauri-apps/api";
import { stringCompare } from "../../utils";

export default function InstanceTiles(props: InstanceTilesProps) {
	let [rows, _] = createResource(async () => {
		let tiles: TileData[] = await invoke("get_instance_tiles", { instanceId: props.instanceId });

		// Sort by ID, then move each tile into a bucket of large or small
		let smallTiles = [];
		let largeTiles = [];
		for (let tile of tiles.sort((x, y) => stringCompare(x.id, y.id))) {
			if (tile.size == "small") {
				smallTiles.push(tile);
			} else {
				largeTiles.push(tile);
			}
		}

		// Now make the rows
		let rows: (TileData | undefined)[][] = [];
		let count = Math.max(smallTiles.length, largeTiles.length);
		for (let i = 0; i < count; i++) {
			if (i >= smallTiles.length) {
				rows.push([undefined, largeTiles[i]]);
			} else if (i >= largeTiles.length) {
				rows.push([smallTiles[i], undefined]);
			} else {
				rows.push([smallTiles[i], largeTiles[i]]);
			}
		}

		return rows;
	}, { initialValue: [] })

	return <div class="cont col instance-tiles">
		<For each={rows()}>
			{(row, i) => {
				let smallTile = <div class="cont instance-tile" innerHTML={row[0] == undefined ? "" : row[0].contents}></div>;
				let largeTile = <div class="cont instance-tile" innerHTML={row[1] == undefined ? "" : row[1].contents}></div>;

				let cls = i() % 2 == 0 ? "small-large" : "large-small";

				// Alternate the order of small and large tiles for more variety
				return <div class={`instance-tile-row ${cls}`}>
					<Switch>
						<Match when={i() % 2 == 0}>
							{smallTile}{largeTile}
						</Match>
						<Match when={i() % 2 == 1}>
							{largeTile}{smallTile}
						</Match>
					</Switch>
				</div>;
			}}
		</For>
	</div>;
}

export interface InstanceTilesProps {
	instanceId: string;
}

interface TileData {
	id: string;
	contents: string;
	size: "small" | "large";
}
