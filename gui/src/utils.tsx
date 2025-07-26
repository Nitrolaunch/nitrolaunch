import { convertFileSrc, invoke } from "@tauri-apps/api/tauri";
import { InstanceIcon, PkgRequest } from "./types";

// Gets the src of an instance icon
export function getInstanceIconSrc(
	icon: InstanceIcon | null | undefined
): string {
	if (icon === null || icon == undefined) {
		return "/icons/default_instance.png";
	} else {
		return convertFileSrc(icon);
	}
}

// Parses a URL query string (after the ?) into an easy to access map
export function parseQueryString(string: string): QueryStringResult {
	if (!string.startsWith("?")) {
		return {};
	}

	string = string.substring(1);
	let entries = string.split("&");
	let out: QueryStringResult = {};
	for (let entry of entries) {
		let items = entry.split("=");
		if (items.length < 2) {
			continue;
		}
		let key = items[0];
		let value = items[1];
		out[key] = value;
	}

	return out;
}

export interface QueryStringResult {
	[key: string]: string | undefined;
}

// Sort comparison function for strings
export function stringCompare(a: string, b: string) {
	return a > b ? 1 : a < b ? -1 : 0;
}

// Makes a string pretty by capitalizing first letters and replacing -,_, and . with spaces
export function beautifyString(string: string) {
	string = string.replace(/\./g, " ");
	string = string.replace(/\-/g, " ");
	string = string.replace(/\_/g, " ");

	// Capitalize
	let last = "";
	for (let i = 0; i < string.length; i++) {
		if (last == "" || last == " ") {
			let left = string.slice(0, i);
			let right = string.slice(i + 1);
			string = left.concat(string[i].toLocaleUpperCase()).concat(right);
		}
		last = string[i];
	}

	return string;
}

// Parses a PkgRequest (repo:id@version) into its parts
export function parsePkgRequest(request: string) {
	let split = request.split(":");
	let repo = split.length > 1 ? split[0] : undefined;
	let right = split[split.length - 1];

	let split2 = right.split("@");
	let version = split2.length > 1 ? split2[1] : undefined;
	let id = split2[0];
	return { id: id, repository: repo, version: version } as PkgRequest;
}

export function pkgRequestToString(request: PkgRequest) {
	let repo = request.repository == undefined ? "" : `${request.repository}:`;
	let version = request.version == undefined ? "" : `@${request.version}`;
	return `${repo}${request.id}${version}`;
}

// Parses a versioned string (object@version) into parts
export function parseVersionedString(
	object: string
): [string, string | undefined] {
	let split = object.split("@");
	return [split[0], split.length > 1 ? split[1] : undefined];
}

// Gets the supported loader list for use with loader selection
export async function getSupportedLoaders(): Promise<string[]> {
	let results: string[] = await invoke("get_supported_loaders");

	results.sort(stringCompare);
	results = ["vanilla"].concat(results);

	return results;
}

// Gets the head icon for a user
export default function getUserIcon(uuid?: string) {
	return uuid == undefined
		? "/default_skin.png"
		: `https://crafatar.com/avatars/${uuid}?overlay`;
}
