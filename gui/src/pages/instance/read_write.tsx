import { invoke } from "@tauri-apps/api/core";
import { getPackageConfigRequest, PackageConfig } from "./PackagesConfig";
import { canonicalizeListOrSingle } from "../../utils/values";
import { Loader } from "../../package";
import { InstanceOrTemplate, Side } from "../../types";
import { beautifyString } from "../../utils";
import { emit } from "@tauri-apps/api/event";
import { ControlData } from "../../components/input/Control";

// Stored configuration for an instance
export interface InstanceConfig {
	from?: string[] | string;
	type?: "client" | "server";
	name?: string;
	icon?: string;
	version?: string | "latest" | "latest_snapshot";
	loader?: ConfiguredLoaders;
	modpack?: string;
	datapack_folder?: string;
	packages?: ConfiguredPackages;
	launch?: LaunchConfig;
	imported?: boolean;
	source_plugin?: string;
	is_editable?: boolean;
	is_deletable?: boolean;
	overrides?: PackageOverrides;
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
	side: Side | undefined,
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
	java?: JavaType;
	[extraKey: string]: any;
}

export type JavaType =
	| "auto"
	| "system"
	| "adoptium"
	| "zulu"
	| "graalvm"
	| string;

export function getJavaDisplayName(x: JavaType) {
	if (x == "auto") {
		return "Auto";
	} else if (x == "system") {
		return "System";
	} else if (x == "adoptium") {
		return "Adoptium";
	} else if (x == "zulu") {
		return "Zulu";
	} else if (x == "graalvm") {
		return "GraalVM";
	} else {
		return beautifyString(x);
	}
}

export interface LaunchMemory {
	min: string;
	max: string;
}

export interface LaunchArgs {
	jvm?: string | string[];
	game?: string | string[];
}

export interface PackageOverrides {
	suppress?: string[];
	force?: string[];
}

// Mode for editing instance-like configs
export enum InstanceConfigMode {
	Instance = "instance",
	Template = "template",
	GlobalTemplate = "base_template",
}

export async function readInstanceConfig(
	id: string | undefined,
	mode: InstanceConfigMode,
) {
	let method =
		mode == InstanceConfigMode.Instance
			? "get_instance_config"
			: mode == InstanceConfigMode.Template
				? "get_template_config"
				: "get_base_template";
	try {
		return (await invoke(method, { id: id })) as InstanceConfig;
	} catch (e) {
		throw e;
	}
}

// Gets the config for an instance or template that can actually be edited and isn't inherited
export async function readEditableInstanceConfig(
	id: string | undefined,
	mode: InstanceConfigMode,
): Promise<[InstanceConfig, { [key: string]: any }]> {
	let method =
		mode == InstanceConfigMode.Instance
			? "get_editable_instance_config"
			: mode == InstanceConfigMode.Template
				? "get_editable_template_config"
				: "get_base_template";
	try {
		let result = (await invoke(method, { id: id })) as {
			config: InstanceConfig;
			plugin_config: { [key: string]: any };
		};
		return [result.config, result.plugin_config];
	} catch (e) {
		throw e;
	}
}

export async function saveInstanceConfig(
	id: string | undefined,
	config: InstanceConfig,
	mode: InstanceConfigMode,
) {
	let method =
		mode == InstanceConfigMode.Instance
			? "write_instance_config"
			: mode == InstanceConfigMode.Template
				? "write_template_config"
				: "write_base_template";

	try {
		await invoke(method, {
			id: id,
			config: config,
		});
	} catch (e) {
		throw "Failed to save instance config: " + e;
	}

	if (mode != InstanceConfigMode.GlobalTemplate) {
		emit("instance_or_template_changed", {
			id: id,
			type: mode,
		} as InstanceOrTemplateChangedEvent);
	}
}

export interface InstanceOrTemplateChangedEvent {
	id: string;
	type: InstanceOrTemplate;
}

// Gets parent template configs for a config
export async function getParentTemplates(
	from: string[] | undefined,
	mode: InstanceConfigMode,
) {
	let parentResults: InstanceConfig[] = [];
	if (mode == InstanceConfigMode.GlobalTemplate) {
		parentResults = [];
	} else if (from == undefined || from.length == 0) {
		let parentResult = await invoke("get_base_template", {});
		parentResults = [parentResult as InstanceConfig];
	} else {
		for (let template of from) {
			let parentResult = await invoke("get_template_config", {
				id: template,
			});
			parentResults.push(parentResult as InstanceConfig);
		}
	}

	return parentResults;
}

// Gets the global, client, and server packages configured on an instance or template
export function getConfigPackages(
	config: InstanceConfig,
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

// Gets the configured packages object to set on an instance or template from each of the package groups
export function createConfiguredPackages(
	global: PackageConfig[],
	client: PackageConfig[],
	server: PackageConfig[],
	isInstance: boolean,
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

// Adds a package to an instance or template
export function addPackage(
	config: InstanceConfig,
	pkg: PackageConfig,
	location: "client" | "server" | "all",
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

export function getDerivedPackages(templates: InstanceConfig[]) {
	let allGlobal: PackageConfig[] = [];
	let allClient: PackageConfig[] = [];
	let allServer: PackageConfig[] = [];
	for (let template of templates) {
		let [global, client, server] = getConfigPackages(template);
		allGlobal = allGlobal.concat(global);
		allClient = allClient.concat(client);
		allServer = allServer.concat(server);
	}

	return [allGlobal, allClient, allServer];
}

// Get the derived value from a list of template configs and a property function
export function getDerivedValue(
	templates: InstanceConfig[],
	property: (template: InstanceConfig) => any | undefined,
) {
	let reversed = templates.concat([]).reverse();
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

// Reads JVM or game args from config into an array
export function readArgs(args: string | string[] | undefined) {
	if (args == undefined) {
		return [];
	} else if (typeof args == "string") {
		return args.split(" ");
	} else {
		return args;
	}
}

// Configuration fields modified with controls
export class ControlledConfig {
	fields: { [key: string]: any };

	constructor(fields: { [key: string]: any } | undefined ) {
		if (fields == undefined) {
			this.fields = {};
		} else {
			this.fields = fields;
		}
	}

	getControl(id: string): any {
		let object = this.fields;

		let i = 0;
		let split = id.split(".");

		// Move recursively down the tree
		for (let key of split) {
			if (i == split.length - 1) {
				return object[key];
			} else {
				// Move down
				let newObject = object[key];
				if (newObject == undefined) {
					return undefined;
				}
				object = newObject;
			}

			i++;
		}

		return undefined;
	}

	setControl(id: string, value: any) {
		let object = this.fields;

		let i = 0;
		let split = id.split(".");

		// Move recursively down the tree
		for (let key of split) {
			if (i == split.length - 1) {
				// Set the value
				object[key] = value;
			} else {
				// Move down
				let newObject = object[key];
				if (newObject == undefined) {
					object[key] = {};
					newObject = object[key];
				}
				object = newObject;
			}

			i++;
		}
	}

	removeControl(id: string) {
		let object = this.fields;

		let i = 0;
		let split = id.split(".");

		// Move recursively down the tree
		for (let key of split) {
			if (i == split.length - 1) {
				// Set the value
				delete object[key];
			} else {
				// Move down
				let newObject = object[key];
				if (newObject == undefined) {
					break;
				}
				object = newObject;
			}

			i++;
		}
	}

	/** Applies these fields on top of a base config, replacing the fields */
	apply(base: { [key: string]: any }) {
		this.fields = deepMerge(base, this.fields);
	}

	/** Cleans up config, removing non-serialized default controls from the final output */
	cleanup(controls: ControlData[]) {
		// First, remove non-serialized controls
		for (let control of controls) {
			if (control.always_serialize) {
				continue;
			}

			let value = this.getControl(control.id);
			if (value == undefined || value == control.default) {
				this.removeControl(control.id);
			}
		}

		// Then, recursively remove any empty objects
		removeEmptyObjects(this.fields);
	}
}

/** Recursively merges two JS objects */
function deepMerge(obj1: { [key: string]: any }, obj2: { [key: string]: any }) {
	const result = { ...obj1 }; // Start with a shallow copy of obj1
	for (const key in obj2) {
		if (
			obj2[key] &&
			typeof obj2[key] === "object" &&
			!Array.isArray(obj2[key])
		) {
			result[key] = deepMerge(result[key] || {}, obj2[key]);
		} else {
			result[key] = obj2[key];
		}
	}
	return result;
}

/** Removes empty objects in this object, returning true if this object is now empty as well */
function removeEmptyObjects(obj: { [key: string]: any }) {
	for (let key in obj) {
		if (
			obj[key] != undefined &&
			typeof obj[key] == "object" &&
			!Array.isArray(obj[key])
		) {
			let result = removeEmptyObjects(obj[key]);
			if (result) {
				delete obj[key];
			}
		}
	}

	return Object.keys(obj).length == 0;
}
