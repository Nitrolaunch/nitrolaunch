import { PackageCategory, PackageType } from "./package";

export type Side = "client" | "server";
export type InstanceIcon = string;
export type InstanceOrProfile = "instance" | "profile";

export interface InstanceInfo {
	id: string;
	name: string | null;
	side: Side;
	icon: InstanceIcon | null;
	pinned: boolean;
	from_plugin: boolean;
	version: string;
}

export type InstanceMap = {
	[id: string]: InstanceInfo;
};

export interface GroupInfo {
	id: string;
	contents: string[];
}

export interface RunningInstanceInfo {
	info: InstanceInfo;
	state: RunState;
}

export type RunState = "not_started" | "preparing" | "running";

export interface UpdateRunStateEvent {
	instance: string;
	state: RunState;
}

export interface AuthDisplayEvent {
	url: string;
	device_code: string;
}

export interface PackageMeta {
	name?: string;
	description?: string;
	long_description?: string;
	banner?: string;
	icon?: string;
	gallery?: string[];
	categories?: PackageCategory[];
	website?: string;
	support_link?: string;
	documentation?: string;
	source?: string;
	issues?: string;
	community?: string;
	license?: string;
	authors?: string[];
	downloads?: number;
}

export interface PackageProperties {
	types?: PackageType[];
	supported_versions?: string[];
	supported_loaders?: string[];
	supported_sides?: Side[];
	content_versions?: string[];
	features?: string[] | string;
}

export interface PkgRequest {
	id: string;
	repository?: string;
	version?: string;
}

export interface PackageSearchResults {
	results: string[];
	total_results: number;
	previews: { [id: string]: [PackageMeta, PackageProperties] };
}

export interface Theme {
	id: string;
	name: string;
	description?: string;
	css: string;
	color: string;
}
