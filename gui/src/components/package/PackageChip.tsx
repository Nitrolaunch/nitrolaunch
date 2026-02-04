import { createMemo, createResource } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { PackageMeta, PkgRequest } from "../../types";
import { useNavigate } from "@solidjs/router";

import "./PackageChip.css";
import { pkgRequestToString } from "../../utils";

export default function PackageChip(props: PackageChipProps) {
	let navigate = useNavigate();

	let pkg = createMemo(() => pkgRequestToString(props.req));

	let [meta, _] = createResource(async () => {
		try {
			return await invoke("get_package_meta", { package: pkg() }) as PackageMeta;
		} catch (e) {
			console.error("Failed to load package: " + e);
			return {};
		}
	}, { initialValue: {} });

	return <div class="cont bubble-hover package-chip" onclick={() => {
		navigate(`/packages/package/${pkg()}`);
	}}>
		<img
			class="package-chip-icon"
			src={meta().icon == undefined ? "icons/default_instance.png" : meta().icon!}
		/>
		{meta().name == undefined ? pkg() : meta().name}
	</div>
}

export interface PackageChipProps {
	req: PkgRequest;
}
