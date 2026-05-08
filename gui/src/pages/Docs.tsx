import { useParams } from "@solidjs/router";

export default function Docs() {
	let params = useParams();

	let subpath = () => params.subpath;

	return (
		<div id="docs">
			<iframe
				src={`https://nitrolaunch.github.io/nitrolaunch/${subpath()}`}
				style="width: 100vw;height:100vh;border:none;margin-left:-0.47rem;margin-top:-0.47rem;"
			></iframe>
		</div>
	);
}
