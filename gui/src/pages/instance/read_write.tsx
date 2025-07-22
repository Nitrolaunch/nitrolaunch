import { invoke } from "@tauri-apps/api";
import { getPackageConfigRequest, PackageConfig } from "./PackagesConfig";
import { canonicalizeListOrSingle } from "../../utils/values";
import { Loader } from "../../package";
import { Side } from "../../types";

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
	launch?: LaunchConfig;
	[extraKey: string]: any;
}

export type ConfiguredLoaders =
	| Loader
	| {
			client?: Loader;
			server?: Loader;
	  };

export function getConfiguredLoader(
	loaders: ConfiguredLoaders | undefined,
	side: Side | undefined
) {
	if (loaders == undefined) {
		return undefined;
	} else if (typeof loaders == "string") {
		return loaders;
	} else {
		return side == "client"
			? loaders.client
			: side == "server"
			? loaders.server
			: undefined;
	}
}

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
	env?: { [key: string]: string };
	java?: "auto" | "system" | "adoptium" | "zulu" | "graalvm" | string;
	[extraKey: string]: any;
}

export interface LaunchMemory {
	min: string;
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
		return (await invoke(method, { id: id })) as InstanceConfig;
	} catch (e) {
		throw e;
	}
}

// Gets the config for an instance or profile that can actually be edited and isn't inherited
export async function readEditableInstanceConfig(
	id: string | undefined,
	mode: InstanceConfigMode
) {
	let method =
		mode == InstanceConfigMode.Instance
			? "get_editable_instance_config"
			: mode == InstanceConfigMode.Profile
			? "get_editable_profile_config"
			: "get_global_profile";
	try {
		return (await invoke(method, { id: id })) as InstanceConfig;
	} catch (e) {
		throw e;
	}
}

export async function saveInstanceConfig(
	id: string | undefined,
	config: InstanceConfig,
	mode: InstanceConfigMode
) {
	let method =
		mode == InstanceConfigMode.Instance
			? "write_instance_config"
			: mode == InstanceConfigMode.Profile
			? "write_profile_config"
			: "write_global_profile";

	try {
		await invoke(method, {
			id: id,
			config: config,
		});
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

export function getDerivedPackages(profiles: InstanceConfig[]) {
	let allGlobal: PackageConfig[] = [];
	let allClient: PackageConfig[] = [];
	let allServer: PackageConfig[] = [];
	for (let profile of profiles) {
		let [global, client, server] = getConfigPackages(profile);
		allGlobal = allGlobal.concat(global);
		allClient = allClient.concat(client);
		allServer = allServer.concat(server);
	}

	return [allGlobal, allClient, allServer];
}

// Get the derived value from a list of profile configs and a property function
export function getDerivedValue(
	profiles: InstanceConfig[],
	property: (profile: InstanceConfig) => any | undefined
) {
	let reversed = profiles.concat([]).reverse();
	return reversed.map(property).find((x) => x != undefined);
}

// Parses launch memory args
export function parseLaunchMemory(memory: string | LaunchMemory | undefined) {
	if (memory == undefined) {
		return [undefined, undefined];
	}

	if (typeof memory == "string") {
		let num = parseMemoryNum(memory);
		return [num, num];
	} else {
		return [parseMemoryNum(memory.min), parseMemoryNum(memory.max)];
	}
}

// Parses a JVM memory number to an amount in megabytes
export function parseMemoryNum(num: string) {
	num = num.toLocaleLowerCase();

	if (num.endsWith("b")) {
		return +num.substring(0, num.length - 1) / 1024 / 1024;
	} else if (num.endsWith("k")) {
		return +num.substring(0, num.length - 1) / 1024;
	} else if (num.endsWith("m")) {
		return +num.substring(0, num.length - 1);
	} else if (num.endsWith("g")) {
		return +num.substring(0, num.length - 1) * 1024;
	} else {
		return +num.substring(0, num.length - 1) / 1024 / 1024;
	}
}
