import { For, Match, Switch } from "solid-js";
import { PkgRequest } from "../../types";
import "./ResolutionError.css";
import PackageChip from "./PackageChip";

// Displays an error during package resolution
export default function ResolutionError(props: ResolutionErrorProps) {
	let data = () => props.error.data as any;

	return (
		<div class="cont col package-resolution-error">
			<Switch>
				<Match when={props.error.type == "package_context"}>
					<div class="cont resolution-error-header">
						In <PackageChip req={data()[0]} />
					</div>
					<ResolutionError error={data()[1]} />
				</Match>
				<Match when={props.error.type == "failed_to_preload"}>
					<div class="cont resolution-error-header">
						Failed to load packages
					</div>
					<pre class="cont full-error">{data()}</pre>
				</Match>
				<Match when={props.error.type == "failed_to_get_properties"}>
					<div class="cont resolution-error-header">
						Failed to get <PackageChip req={data()[0]} />
					</div>
					<pre class="cont full-error">{data()[1]}</pre>
				</Match>
				<Match when={props.error.type == "no_valid_versions_found"}>
					<div class="cont">
						No valid versions found for <PackageChip req={data()[0]} />
						Requested versions: {data()[1].toString()}
					</div>
				</Match>
				<Match when={props.error.type == "extension_not_fulfilled"}>
					<div class="cont">
						<Switch>
							<Match when={data()[0] == undefined}>
								A package
							</Match>
							<Match when={data()[0] != undefined}>
								<PackageChip req={data()[0]} />
							</Match>
						</Switch>
						extends the functionality of
						<PackageChip req={data()[1]} />
						, which is not installed
					</div>
				</Match>
				<Match when={props.error.type == "explicit_require_not_fulfilled"}>
					<div class="cont">
						<PackageChip req={data()[0]} />
						has been explicitly required by
						<PackageChip req={data()[1]} />
						. This means it must be required by the user in their config.`
					</div>
				</Match>
				<Match when={props.error.type == "incompatible_package"}>
					<div class="cont">
						<PackageChip req={data()[0]} />
						is incompatible with the packages
						<For each={data()[1]}>
							{(pkg) =>
								<PackageChip req={pkg} />
							}
						</For>
					</div>
				</Match>
				<Match when={props.error.type == "failed_to_evaluate"}>
					<div class="cont resolution-error-header">
						Failed to evaluate package package
						<PackageChip req={data()[0]} />
					</div>
					<pre class="cont full-error">{data()[1]}</pre>
				</Match>
				<Match when={props.error.type == "misc"}>
					<div class="cont resolution-error-header">Other error:</div>
					<div class="cont full-error">{data()}</div>
				</Match>
			</Switch>
		</div>
	);
}

export interface ResolutionErrorProps {
	error: ResolutionErrorData;
}

// Data for the actual error
export type ResolutionErrorData =
	| {
		type: "package_context";
		data: [PkgRequest, ResolutionErrorData];
	}
	| {
		type: "failed_to_preload";
		data: string;
	}
	| {
		type: "failed_to_get_properties";
		data: [PkgRequest, string];
	}
	| {
		type: "no_valid_versions_found";
		data: [PkgRequest, string[]];
	}
	| {
		type: "extension_not_fulfilled";
		data: [PkgRequest | undefined, PkgRequest];
	}
	| {
		type: "explicit_require_not_fulfilled";
		data: [PkgRequest, PkgRequest];
	}
	| {
		type: "incompatible_package";
		data: [PkgRequest, string[]];
	}
	| {
		type: "failed_to_evaluate";
		data: [PkgRequest, string];
	}
	| {
		type: "misc";
		data: string;
	};
