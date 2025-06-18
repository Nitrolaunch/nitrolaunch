import { invoke } from "@tauri-apps/api";
import { PackageConfig } from "./PackagesConfig";

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
	| string
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
