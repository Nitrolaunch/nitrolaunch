use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use mcvm_core::io::java::args::MemoryNum;
use mcvm_core::util::versions::MinecraftVersionDeser;
use mcvm_pkg::overrides::PackageOverrides;
use mcvm_shared::addon::AddonKind;
use mcvm_shared::loaders::Loader;
use mcvm_shared::pkg::PackageStability;
use mcvm_shared::util::{merge_options, DefaultExt, DeserListOrSingle};
use mcvm_shared::versions::{VersionInfo, VersionPattern};
use mcvm_shared::Side;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::package::PackageConfigDeser;

/// Configuration for an instance
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct InstanceConfig {
	/// The type or side of this instance
	#[serde(rename = "type")]
	pub side: Option<Side>,
	/// The display name of this instance
	#[serde(default)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	/// A path to an icon file for this instance
	#[serde(default)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub icon: Option<String>,
	/// The common config of this instance
	#[serde(flatten)]
	pub common: CommonInstanceConfig,
	/// Window configuration
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub window: ClientWindowConfig,
}

impl InstanceConfig {
	/// Merge this config with another one, with right side taking precendence
	pub fn merge(&mut self, other: Self) {
		self.common.merge(other.common);
		self.icon = other.icon.or(self.icon.clone());
		self.side = other.side.or(self.side);
		self.window.merge(other.window);
	}
}

/// Common full instance config for both client and server
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(default)]
pub struct CommonInstanceConfig {
	/// A profile to use
	#[serde(skip_serializing_if = "DeserListOrSingle::is_empty")]
	pub from: DeserListOrSingle<String>,
	/// The Minecraft version
	pub version: Option<MinecraftVersionDeser>,
	/// Configured loader
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub loader: Option<String>,
	/// Default stability setting of packages on this instance
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub package_stability: Option<PackageStability>,
	/// Launch configuration
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub launch: LaunchConfig,
	/// The folder for global datapacks to be installed to
	#[serde(skip_serializing_if = "Option::is_none")]
	pub datapack_folder: Option<String>,
	/// Packages for this instance
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub packages: Vec<PackageConfigDeser>,
	/// Overrides for packages on this instance
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub overrides: PackageOverrides,
	/// Config for plugins
	#[serde(flatten)]
	#[serde(skip_serializing_if = "serde_json::Map::is_empty")]
	pub plugin_config: serde_json::Map<String, serde_json::Value>,
}

impl CommonInstanceConfig {
	/// Merge multiple common configs
	pub fn merge(&mut self, other: Self) -> &mut Self {
		self.from.merge(other.from);
		self.version = other.version.or(self.version.clone());
		self.loader = other.loader.or(self.loader.clone());
		self.package_stability = other.package_stability.or(self.package_stability);
		self.launch.merge(other.launch);
		self.datapack_folder = other.datapack_folder.or(self.datapack_folder.clone());
		self.packages.extend(other.packages);
		self.overrides.suppress.extend(other.overrides.suppress);
		mcvm_core::util::json::merge_objects(&mut self.plugin_config, other.plugin_config);

		self
	}
}

/// Different representations for JVM / game arguments
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum Args {
	/// A list of separate arguments
	List(Vec<String>),
	/// A single string of arguments
	String(String),
}

impl Args {
	/// Parse the arguments into a vector
	pub fn parse(&self) -> Vec<String> {
		match self {
			Self::List(vec) => vec.clone(),
			Self::String(string) => string.split(' ').map(|string| string.to_string()).collect(),
		}
	}

	/// Merge Args
	pub fn merge(&mut self, other: Self) {
		let mut out = self.parse();
		out.extend(other.parse());
		*self = Self::List(out);
	}
}

impl Default for Args {
	fn default() -> Self {
		Self::List(Vec::new())
	}
}

/// Arguments for the process when launching
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct LaunchArgs {
	/// Arguments for the JVM
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub jvm: Args,
	/// Arguments for the game
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub game: Args,
}

/// Different representations of both memory arguments for the JVM
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum LaunchMemory {
	/// No memory arguments
	#[default]
	None,
	/// A single memory argument shared for both
	Single(String),
	/// Different memory arguments for both
	Both {
		/// The minimum memory
		min: String,
		/// The maximum memory
		max: String,
	},
}

impl LaunchMemory {
	/// Parse this memory as a minimum and maximum memory
	pub fn to_min_max(self) -> (Option<MemoryNum>, Option<MemoryNum>) {
		let min_mem = match &self {
			LaunchMemory::None => None,
			LaunchMemory::Single(string) => MemoryNum::parse(string),
			LaunchMemory::Both { min, .. } => MemoryNum::parse(min),
		};
		let max_mem = match &self {
			LaunchMemory::None => None,
			LaunchMemory::Single(string) => MemoryNum::parse(string),
			LaunchMemory::Both { max, .. } => MemoryNum::parse(max),
		};

		(min_mem, max_mem)
	}
}

fn default_java() -> String {
	"auto".into()
}

/// Options for the Minecraft QuickPlay feature
#[derive(Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum QuickPlay {
	/// QuickPlay a world
	World {
		/// The world to play
		world: String,
	},
	/// QuickPlay a server
	Server {
		/// The server address to join
		server: String,
		/// The port for the server to connect to
		port: Option<u16>,
	},
	/// QuickPlay a realm
	Realm {
		/// The realm name to join
		realm: String,
	},
	/// Don't do any QuickPlay
	#[default]
	None,
}

/// Configuration for the launching of the game
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct LaunchConfig {
	/// The arguments for the process
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub args: LaunchArgs,
	/// JVM memory options
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub memory: LaunchMemory,
	/// The java installation to use
	#[serde(default = "default_java")]
	pub java: String,
	/// Environment variables
	#[serde(default)]
	#[serde(skip_serializing_if = "HashMap::is_empty")]
	pub env: HashMap<String, String>,
	/// A wrapper command
	#[serde(default)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub wrapper: Option<WrapperCommand>,
	/// QuickPlay options
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub quick_play: QuickPlay,
	/// Whether or not to use the Log4J configuration
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub use_log4j_config: bool,
}

impl LaunchConfig {
	/// Merge multiple LaunchConfigs
	pub fn merge(&mut self, other: Self) -> &mut Self {
		self.args.jvm.merge(other.args.jvm);
		self.args.game.merge(other.args.game);
		if !matches!(other.memory, LaunchMemory::None) {
			self.memory = other.memory;
		}
		self.java = other.java;
		self.env.extend(other.env);
		if other.wrapper.is_some() {
			self.wrapper = other.wrapper;
		}
		if !matches!(other.quick_play, QuickPlay::None) {
			self.quick_play = other.quick_play;
		}

		self
	}
}

impl Default for LaunchConfig {
	fn default() -> Self {
		Self {
			args: LaunchArgs {
				jvm: Args::default(),
				game: Args::default(),
			},
			memory: LaunchMemory::default(),
			java: default_java(),
			env: HashMap::new(),
			wrapper: None,
			quick_play: QuickPlay::default(),
			use_log4j_config: false,
		}
	}
}

/// A wrapper command
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct WrapperCommand {
	/// The command to run
	pub cmd: String,
	/// The command's arguments
	#[serde(default)]
	pub args: Vec<String>,
}

/// Resolution for a client window
#[derive(Deserialize, Serialize, Clone, Debug, Copy, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct WindowResolution {
	/// The width of the window
	pub width: u32,
	/// The height of the window
	pub height: u32,
}

/// Configuration for the client window
#[derive(Deserialize, Serialize, Default, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(default)]
pub struct ClientWindowConfig {
	/// The resolution of the window
	#[serde(skip_serializing_if = "Option::is_none")]
	pub resolution: Option<WindowResolution>,
}

impl ClientWindowConfig {
	/// Merge two ClientWindowConfigs
	pub fn merge(&mut self, other: Self) -> &mut Self {
		self.resolution = merge_options(self.resolution, other.resolution);
		self
	}
}

/// Checks if an instance ID is valid
pub fn is_valid_instance_id(id: &str) -> bool {
	for c in id.chars() {
		if !c.is_ascii() {
			return false;
		}

		if c.is_ascii_punctuation() {
			match c {
				'_' | '-' | '.' | ':' => {}
				_ => return false,
			}
		}

		if c.is_ascii_whitespace() {
			return false;
		}
	}

	true
}

/// Check if a loader can be installed by MCVM
pub fn can_install_loader(loader: &Loader) -> bool {
	matches!(loader, Loader::Vanilla)
}

/// Get the paths on an instance to put addons in
pub fn get_addon_paths(
	instance: &InstanceConfig,
	game_dir: &Path,
	addon: AddonKind,
	selected_worlds: &[String],
	version_info: &VersionInfo,
) -> anyhow::Result<Vec<PathBuf>> {
	let side = instance.side.context("Instance side missing")?;
	Ok(match addon {
		AddonKind::ResourcePack => {
			if side == Side::Client {
				// Resource packs are texture packs on older versions
				if VersionPattern::After("13w24a".into()).matches_info(version_info) {
					vec![game_dir.join("resourcepacks")]
				} else {
					vec![game_dir.join("texturepacks")]
				}
			} else {
				vec![game_dir.join("resourcepacks")]
			}
		}
		AddonKind::Mod => vec![game_dir.join("mods")],
		AddonKind::Plugin => {
			if side == Side::Server {
				vec![game_dir.join("plugins")]
			} else {
				vec![]
			}
		}
		AddonKind::Shader => {
			if side == Side::Client {
				vec![game_dir.join("shaderpacks")]
			} else {
				vec![]
			}
		}
		AddonKind::Datapack => {
			if let Some(datapack_folder) = &instance.common.datapack_folder {
				vec![game_dir.join(datapack_folder)]
			} else {
				match side {
					Side::Client => {
						let saves_dir = game_dir.join("saves");
						if saves_dir.exists() {
							game_dir
								.join("saves")
								.read_dir()
								.context("Failed to read saves directory")?
								.filter_map(|world| {
									let world = world.ok()?;
									let path = world.path();
									// Filter worlds not in the list
									if !selected_worlds.is_empty() {
										let dir_name = path.file_name()?.to_string_lossy();
										if !selected_worlds.iter().any(|x| x == dir_name.as_ref()) {
											return None;
										}
									}
									Some(path.join("datapacks"))
								})
								.collect()
						} else {
							vec![]
						}
					}
					Side::Server => {
						// TODO: Support custom world names
						vec![game_dir.join("world").join("datapacks")]
					}
				}
			}
		}
	})
}
