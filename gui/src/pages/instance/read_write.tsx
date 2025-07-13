import { invoke } from "@tauri-apps/api";
import { getPackageConfigRequest, PackageConfig } from "./PackagesConfig";
import { canonicalizeListOrSingle } from "../../utils/values";
import { Loader } from "../../package";

// Stored configuration for an instance
export interface InstanceConfig {
	from?: string[];
	type?: "client" | "server";
	name?: string;
	icon?: string;
	version?: string | "latest" | "latest_snapshot";
	loader?: ConfiguredLoaders;
	datapack_folder?: string;
	packages?: ConfiguredPackages;
	[extraKey: string]: any;
}

export type ConfiguredLoaders =
	| Loader
	| {
			client?: string;
			server?: string;
	  };

export type ConfiguredPackages =
	| PackageConfig[]
	| {
			global?: PackageConfig[];
			client?: PackageConfig[];
			server?: PackageConfig[];
	  };

export interface LaunchConfig {
	memory?: string | LaunchMemory;
	args?: LaunchArgs;
	env?: string[];
	java?: "auto" | "system" | "adoptium" | "zulu" | "graalvm" | string;
	[extraKey: string]: any;
}

export interface LaunchMemory {
	init: string;
	max: string;
}

export interface LaunchArgs {
	jvm?: string | string[];
	game?: string | string[];
}

// Mode for editing instance-like configs
export enum InstanceConfigMode {
	Instance = "instance",
	Profile = "profile",
	GlobalProfile = "global_profile",
}

export async function readInstanceConfig(
	id: string | undefined,
	mode: InstanceConfigMode
) {
	let method =
		mode == InstanceConfigMode.Instance
			? "get_instance_config"
			: mode == InstanceConfigMode.Profile
			? "get_profile_config"
			: "get_global_profile";
	try {
		let result = await invoke(method, { id: id });
		let configuration = result as InstanceConfig;

		return configuration;
	} catch (e) {
		throw e;
	}
}

export async function saveInstanceConfig(
	id: string | undefined,
	config: InstanceConfig,
	mode: InstanceConfigMode
) {
	try {
		if (mode == InstanceConfigMode.Instance) {
			await invoke("write_instance_config", {
				id: id,
				config: config,
			});
		} else if (mode == InstanceConfigMode.Profile) {
			await invoke("write_profile_config", {
				id: id,
				config: config,
			});
		} else if (mode == InstanceConfigMode.GlobalProfile) {
			await invoke("write_global_profile", { config: config });
		} else {
		}
	} catch (e) {
		throw "Failed to save instance config: " + e;
	}
}

// Gets the global, client, and server packages configured on an instance or profile
export function getConfigPackages(
	config: InstanceConfig
): [PackageConfig[], PackageConfig[], PackageConfig[]] {
	if (config.packages == undefined) {
		return [[], [], []];
	} else if (length in config.packages) {
		return [config.packages as PackageConfig[], [], []];
	} else {
		let packages = config.packages! as any;

		return [
			packages.global == undefined ? [] : packages.global,
			packages.client == undefined ? [] : packages.client,
			packages.server == undefined ? [] : packages.server,
		];
	}
}

// Gets the configured packages object to set on an instance or profile from each of the package groups
export function createConfiguredPackages(
	global: PackageConfig[],
	client: PackageConfig[],
	server: PackageConfig[],
	isInstance: boolean
): ConfiguredPackages {
	if (isInstance) {
		return global;
	} else {
		// Only include the global list if we don't need the other ones
		if (client.length == 0 && server.length == 0) {
			return global;
		} else {
			return {
				global: global,
				client: client,
				server: server,
			};
		}
	}
}

// Adds a package to an instance or profile
export function addPackage(
	config: InstanceConfig,
	pkg: PackageConfig,
	location: "client" | "server" | "all"
) {
	let req = getPackageConfigRequest(pkg);

	let removeExisting = (packages: PackageConfig[]) => {
		packages = packages.filter((x) => getPackageConfigRequest(x).id != req.id);
	};

	if (location == "all") {
		if (config.packages == undefined) {
			config.packages = [pkg];
		} else if (Array.isArray(config.packages)) {
			removeExisting(config.packages);
			config.packages.push(pkg);
		} else {
			removeExisting(canonicalizeListOrSingle(config.packages.global));
			config.packages.global!.push(pkg);
		}
	} else if (location == "client") {
		if (config.packages == undefined) {
			config.packages = { client: [pkg] };
		} else if (Array.isArray(config.packages)) {
			config.packages = { global: config.packages, client: [] };
			removeExisting(config.packages.client!);
			config.packages.client!.push(pkg);
		} else {
			removeExisting(canonicalizeListOrSingle(config.packages.client));
			config.packages.client!.push(pkg);
		}
	} else if (location == "server") {
		if (config.packages == undefined) {
			config.packages = { server: [pkg] };
		} else if (Array.isArray(config.packages)) {
			config.packages = { global: config.packages, server: [] };
			removeExisting(config.packages.server!);
			config.packages.server!.push(pkg);
		} else {
			removeExisting(canonicalizeListOrSingle(config.packages.server));
			config.packages.server!.push(pkg);
		}
	}
}
