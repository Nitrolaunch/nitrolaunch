import {
	AngleRight,
	Animal,
	Audio,
	Book,
	Box,
	Couch,
	CurlyBraces,
	Folder,
	Fullscreen,
	Gear,
	Globe,
	Graph,
	Home,
	Honeycomb,
	Jigsaw,
	Key,
	Language,
	Link,
	MapPin,
	Microphone,
	Minecraft,
	Moon,
	Palette,
	Picture,
	Plus,
	Properties,
	Search,
	Speed,
	Star,
	Sun,
	Sword,
	Text,
	User,
	Window,
} from "./icons";
import { Side } from "./types";
import { beautifyString } from "./utils";

export interface RepoInfo {
	id: string;
	meta: RepoMetadata;
}

export interface RepoMetadata {
	name?: string;
	description?: string;
	nitro_verseion?: string;
	color?: string;
	text_color?: string;
	package_types?: PackageType[];
	package_categories?: PackageCategory[];
}

// We store versions kinda backwards from declarative packages, grouping addons into each content version
export interface PackageVersion {
	id?: string;
	name?: string;
	addons: PackageAddon[];
	relations?: DeclarativePackageRelations;
	minecraft_versions?: string | string[];
	side?: Side;
	loaders?: Loader | Loader[];
	stability?: "stable" | "latest";
	features?: string | string[];
	operating_systems?: string | string[];
	architectures?: string | string[];
	languages?: string | string[];
}

export interface PackageAddon {
	id: string;
	kind: AddonKind;
}

export interface DeclarativePackage {
	relations?: DeclarativePackageRelations;
	addons?: { [id: string]: DeclarativeAddon };
}

export interface DeclarativeAddon {
	kind: AddonKind;
	versions?: DeclarativeAddonVersion[];
}

export interface DeclarativeAddonVersion {
	version: string;
	relations: DeclarativePackageRelations;
	content_versions?: string | string[];
	minecraft_versions?: string | string[];
	side?: Side;
	loaders?: Loader | Loader[];
	stability?: "stable" | "latest";
	features?: string | string[];
	operating_systems?: string | string[];
	architectures?: string | string[];
	languages?: string | string[];
}

export interface DeclarativePackageRelations {
	dependencies?: string[] | string;
	explicit_dependencies?: string[] | string;
	conflicts?: string[] | string;
	extensions?: string[] | string;
	bundled?: string[] | string;
	compats?: [string, string][] | [string, string];
	recommendations?:
		| { value: string; invert?: boolean }[]
		| { value: string; invert?: boolean };
}

export type AddonKind =
	| "mod"
	| "resource_pack"
	| "datapack"
	| "shader"
	| "plugin";

export type PackageType = AddonKind | "bundle";

export enum PackageCategory {
	Adventure = "adventure",
	Atmosphere = "atmosphere",
	Audio = "audio",
	Blocks = "blocks",
	Building = "building",
	Cartoon = "cartoon",
	Challenge = "challenge",
	Combat = "combat",
	Compatability = "compatability",
	Decoration = "decoration",
	Economy = "economy",
	Entities = "entities",
	Equipment = "equipment",
	Exploration = "exploration",
	Extensive = "extensive",
	Fantasy = "fantasy",
	Fonts = "fonts",
	Food = "food",
	GameMechanics = "game_mechanics",
	Gui = "gui",
	Items = "items",
	Language = "language",
	Library = "library",
	Lightweight = "lightweight",
	Magic = "magic",
	Minigame = "minigame",
	Mobs = "mobs",
	Multiplayer = "multiplayer",
	Optimization = "optimization",
	Realistic = "realistic",
	Simplistic = "simplistic",
	Space = "space",
	Social = "social",
	Storage = "storage",
	Structures = "structures",
	Technology = "technology",
	Transportation = "transportation",
	Tweaks = "tweaks",
	Utility = "utility",
	VanillaPlus = "vanilla_plus",
	Worldgen = "worldgen",
}

export function packageCategoryDisplayName(category: PackageCategory) {
	switch (category) {
		case PackageCategory.Adventure:
			return "Adventure";
		case PackageCategory.Atmosphere:
			return "Atmosphere";
		case PackageCategory.Audio:
			return "Audio";
		case PackageCategory.Blocks:
			return "Blocks";
		case PackageCategory.Building:
			return "Building";
		case PackageCategory.Cartoon:
			return "Cartoon";
		case PackageCategory.Challenge:
			return "Challenge";
		case PackageCategory.Combat:
			return "Combat";
		case PackageCategory.Compatability:
			return "Compatability";
		case PackageCategory.Decoration:
			return "Decoration";
		case PackageCategory.Economy:
			return "Economy";
		case PackageCategory.Entities:
			return "Entities";
		case PackageCategory.Equipment:
			return "Equipment";
		case PackageCategory.Exploration:
			return "Exploration";
		case PackageCategory.Extensive:
			return "Extensive";
		case PackageCategory.Fantasy:
			return "Fantasy";
		case PackageCategory.Fonts:
			return "Fonts";
		case PackageCategory.Food:
			return "Food";
		case PackageCategory.GameMechanics:
			return "Game Mechanics";
		case PackageCategory.Gui:
			return "Gui";
		case PackageCategory.Items:
			return "Items";
		case PackageCategory.Language:
			return "Language";
		case PackageCategory.Library:
			return "Library";
		case PackageCategory.Lightweight:
			return "Lightweight";
		case PackageCategory.Magic:
			return "Magic";
		case PackageCategory.Minigame:
			return "Minigame";
		case PackageCategory.Mobs:
			return "Mobs";
		case PackageCategory.Multiplayer:
			return "Multiplayer";
		case PackageCategory.Optimization:
			return "Optimization";
		case PackageCategory.Realistic:
			return "Realistic";
		case PackageCategory.Simplistic:
			return "Simplistic";
		case PackageCategory.Space:
			return "Space";
		case PackageCategory.Social:
			return "Social";
		case PackageCategory.Storage:
			return "Storage";
		case PackageCategory.Structures:
			return "Structures";
		case PackageCategory.Technology:
			return "Technology";
		case PackageCategory.Transportation:
			return "Transportation";
		case PackageCategory.Tweaks:
			return "Tweaks";
		case PackageCategory.Utility:
			return "Utility";
		case PackageCategory.VanillaPlus:
			return "Vanilla+";
		case PackageCategory.Worldgen:
			return "Worldgen";
	}

	return beautifyString(category);
}

export function packageCategoryIcon(category: PackageCategory) {
	switch (category) {
		case PackageCategory.Adventure:
			return Minecraft;
		case PackageCategory.Atmosphere:
			return Sun;
		case PackageCategory.Audio:
			return Audio;
		case PackageCategory.Blocks:
			return Box;
		case PackageCategory.Building:
			return Honeycomb;
		case PackageCategory.Cartoon:
			return Palette;
		case PackageCategory.Challenge:
			return Star;
		case PackageCategory.Combat:
			return Sword;
		case PackageCategory.Compatability:
			return Link;
		case PackageCategory.Decoration:
			return Couch;
		case PackageCategory.Economy:
			return Graph;
		case PackageCategory.Entities:
			return Animal;
		case PackageCategory.Equipment:
			return Key;
		case PackageCategory.Exploration:
			return Search;
		case PackageCategory.Extensive:
			return Home;
		case PackageCategory.Fantasy:
			return Star;
		case PackageCategory.Fonts:
			return Text;
		case PackageCategory.Food:
			return Box;
		case PackageCategory.GameMechanics:
			return Gear;
		case PackageCategory.Gui:
			return Window;
		case PackageCategory.Items:
			return Box;
		case PackageCategory.Language:
			return Language;
		case PackageCategory.Library:
			return Book;
		case PackageCategory.Lightweight:
			return Fullscreen;
		case PackageCategory.Magic:
			return Star;
		case PackageCategory.Minigame:
			return Jigsaw;
		case PackageCategory.Mobs:
			return Animal;
		case PackageCategory.Multiplayer:
			return User;
		case PackageCategory.Optimization:
			return Speed;
		case PackageCategory.Realistic:
			return Picture;
		case PackageCategory.Simplistic:
			return Fullscreen;
		case PackageCategory.Space:
			return Moon;
		case PackageCategory.Social:
			return Microphone;
		case PackageCategory.Storage:
			return Folder;
		case PackageCategory.Structures:
			return MapPin;
		case PackageCategory.Technology:
			return Properties;
		case PackageCategory.Transportation:
			return AngleRight;
		case PackageCategory.Tweaks:
			return Properties;
		case PackageCategory.Utility:
			return Gear;
		case PackageCategory.VanillaPlus:
			return Plus;
		case PackageCategory.Worldgen:
			return Globe;
	}

	return Box;
}

export enum Loader {
	Fabric = "fabric",
	Quilt = "quilt",
	Forge = "forge",
	NeoForge = "neoforged",
	Sponge = "sponge",
	SpongeForge = "spongeforge",
	Paper = "paper",
	Folia = "folia",
}

export function getAllLoaders(modifications: string[]) {
	let out: Loader[] = [];
	for (let loaderMatch of modifications) {
		for (let newLoader of getLoaders(loaderMatch)) {
			if (!out.includes(newLoader)) {
				out.push(newLoader);
			}
		}
	}
	return out;
}

export function getLoaders(modification: string) {
	if (modification == "fabriclike") {
		return [Loader.Fabric, Loader.Quilt];
	} else if (modification == "forgelike") {
		return [Loader.Forge, Loader.SpongeForge];
	}
	return [modification as Loader];
}

export function getLoaderDisplayName(loader: Loader) {
	if (loader == "fabric") {
		return "Fabric";
	} else if (loader == "quilt") {
		return "Quilt";
	} else if (loader == "forge") {
		return "Forge";
	} else if (loader == "neoforged") {
		return "NeoForged";
	} else if (loader == "sponge") {
		return "Sponge";
	} else if (loader == "spongeforge") {
		return "SpongeForge";
	} else if (loader == "paper") {
		return "Paper";
	} else if (loader == "folia") {
		return "Folia";
	} else {
		return beautifyString(loader);
	}
}

export function getLoaderImage(loader: Loader) {
	if (loader == "fabric") {
		return "/icons/fabric.png";
	} else if (loader == "quilt") {
		return "/icons/quilt.png";
	} else if (loader == "forge") {
		return "/icons/forge.png";
	} else if (loader == "neoforged") {
		return "/icons/neoforge.png";
	} else if (loader == "sponge") {
		return "/icons/sponge.png";
	} else if (loader == "spongeforge") {
		return "/icons/sponge.png";
	} else if (loader == "paper" || loader == "folia") {
		return "/icons/paper.png";
	} else {
		return "/icons/default_instance.png";
	}
}

export function getLoaderColor(loader: Loader) {
	if (loader == "fabric") {
		return "#d4c9af";
	} else if (loader == "quilt") {
		return "#dc29dd";
	} else if (loader == "forge") {
		return "#505c74";
	} else if (loader == "neoforged") {
		return "#d6732f";
	} else if (loader == "sponge") {
		return "#f8ce0f";
	} else if (loader == "spongeforge") {
		return "#f8ce0f";
	} else if (loader == "paper") {
		return "#fbfbfb";
	} else if (loader == "folia") {
		return "#ff6576";
	} else {
		return "var(--fg2)";
	}
}

export function getLoaderSide(loader: string | undefined) {
	if (
		loader == undefined ||
		loader == "vanilla" ||
		loader == "fabric" ||
		loader == "quilt" ||
		loader == "forge" ||
		loader == "neoforged" ||
		loader == "rift" ||
		loader == "risugamis"
	) {
		return undefined;
	} else if (loader == "liteloader") {
		return "client";
	} else {
		return "server";
	}
}

export function getPackageTypeDisplayName(type: PackageType) {
	if (type == "mod") {
		return "Mod";
	} else if (type == "resource_pack") {
		return "Resource Pack";
	} else if (type == "datapack") {
		return "Datapack";
	} else if (type == "plugin") {
		return "Plugin";
	} else if (type == "shader") {
		return "Shader";
	} else if (type == "bundle") {
		return "Bundle";
	} else {
		return beautifyString(type);
	}
}

export function getPackageTypeColor(type: PackageType) {
	if (type == "mod") {
		return "var(--instance)";
	} else if (type == "resource_pack") {
		return "var(--profile)";
	} else if (type == "datapack") {
		return "var(--package)";
	} else if (type == "plugin") {
		return "var(--pluginfg)";
	} else if (type == "shader") {
		return "var(--warning)";
	} else if (type == "bundle") {
		return "var(--fg2)";
	} else {
		return "var(--fg2)";
	}
}

export function getPackageTypeIcon(type: PackageType) {
	if (type == "mod") {
		return Box;
	} else if (type == "resource_pack") {
		return Palette;
	} else if (type == "datapack") {
		return CurlyBraces;
	} else if (type == "plugin") {
		return Jigsaw;
	} else if (type == "shader") {
		return Sun;
	} else if (type == "bundle") {
		return Folder;
	} else {
		return Box;
	}
}
