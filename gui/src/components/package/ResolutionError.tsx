import { Match, Switch } from "solid-js";
import { PkgRequest } from "../../types";
import { pkgRequestToString } from "../../utils";
import "./ResolutionError.css";

// Displays an error during package resolution
export default function ResolutionError(props: ResolutionErrorProps) {
	let data = () => props.error.data as any;

	return (
		<div class="cont col package-resolution-error">
			<Switch>
				<Match when={props.error.type == "package_context"}>
					<div class="cont resolution-error-header">
						In package {pkgRequestToString(data()[0])}
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
						Failed to get package {pkgRequestToString(data()[0])}
					</div>
					<pre class="cont full-error">{data()[1]}</pre>
				</Match>
				<Match when={props.error.type == "no_valid_versions_found"}>
					<div class="cont">
						No valid versions found for package {pkgRequestToString(data()[0])}
						Requested versions: {data()[1].toString()}
					</div>
				</Match>
				<Match when={props.error.type == "extension_not_fulfilled"}>
					<div class="cont">
						{data()[0] == undefined
							? "A package"
							: `The package ${pkgRequestToString(data()[0])}`}
						{` extends the functionality of the package ${pkgRequestToString(
							data()[1],
						)}, which is not installed`}
					</div>
				</Match>
				<Match when={props.error.type == "explicit_require_not_fulfilled"}>
					<div class="cont">
						{`Package ${pkgRequestToString(
							data()[0],
						)} has been explicitly required by package ${pkgRequestToString(
							data()[1],
						)}. This means it must be required by the user in their config.`}
					</div>
				</Match>
				<Match when={props.error.type == "incompatible_package"}>
					<div class="cont">
						{`Package ${pkgRequestToString(
							data()[0],
						)} is incompatible with the packages ${data()[1].join(",")}`}
					</div>
				</Match>
				<Match when={props.error.type == "failed_to_evaluate"}>
					<div class="cont resolution-error-header">
						Failed to evaluate package package {pkgRequestToString(data()[0])}
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
