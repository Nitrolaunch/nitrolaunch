use std::collections::{HashMap, HashSet};

use nitro_config::instance::InstanceConfig;
use nitro_config::template::TemplateConfig;
use nitro_pkg::repo::{PackageFlag, RepoMetadata};
use nitro_pkg::script_eval::AddonInstructionData;
use nitro_pkg::{PackageContentType, PackageSearchResults, RecommendedPackage, RequiredPackage};
use nitro_shared::addon::AddonKind;
use nitro_shared::id::{InstanceID, TemplateID};
use nitro_shared::lang::translate::LanguageMap;
use nitro_shared::loaders::Loader;
use nitro_shared::minecraft::MinecraftUserProfile;
use nitro_shared::minecraft::VersionEntry;
use nitro_shared::pkg::{PackageID, PackageSearchParameters};
use nitro_shared::versions::VersionPattern;
use nitro_shared::UpdateDepth;
use nitro_shared::{versions::VersionInfo, Side};
use serde::{Deserialize, Serialize};

use super::Hook;

macro_rules! def_hook {
	($struct:ident, $name:literal, $desc:literal, $arg:ty, $res:ty, $version:literal, $asynchronous:literal, $($extra:tt)*) => {
		#[doc = $desc]
		pub struct $struct;

		impl Hook for $struct {
			type Arg = $arg;
			type Result = $res;

			fn get_name_static() -> &'static str {
				$name
			}

			fn get_version() -> u16 {
				$version
			}

			fn is_asynchronous() -> bool {
				$asynchronous
			}

			$(
				$extra
			)*
		}
	};

	($struct:ident, $name:literal, $desc:literal, $arg:ty, $res:ty, $version:literal, $($extra:tt)*) => {
		def_hook!($struct, $name, $desc, $arg, $res, $version, false, $($extra)*);
	};
}

def_hook!(
	OnLoad,
	"on_load",
	"Hook for when a plugin is loaded",
	(),
	(),
	1,
	true,
);

def_hook!(
	StartWorker,
	"start_worker",
	"Hook for starting a long-running worker alongside the plugin runner",
	(),
	(),
	1,
	true,
);

def_hook!(
	Subcommand,
	"subcommand",
	"Hook for when a command's subcommands are run",
	Vec<String>,
	(),
	1,
	fn get_takes_over() -> bool {
		true
	}
);

def_hook!(
	ModifyInstanceConfig,
	"modify_instance_config",
	"Hook for modifying an instance's configuration",
	ModifyInstanceConfigArgument,
	ModifyInstanceConfigResult,
	2,
	true,
);

/// Argument to the ModifyInstanceConfig hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModifyInstanceConfigArgument {
	/// The instance's configuration
	pub config: InstanceConfig,
}

/// Result from the ModifyInstanceConfig hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModifyInstanceConfigResult {
	/// Configuration to apply
	pub config: InstanceConfig,
}

def_hook!(
	AddVersions,
	"add_versions",
	"Hook for adding extra versions to the version manifest",
	UpdateDepth,
	Vec<VersionEntry>,
	2,
	true,
);

def_hook!(
	OnInstanceSetup,
	"on_instance_setup",
	"Hook for doing work when setting up an instance for update or launch",
	OnInstanceSetupArg,
	OnInstanceSetupResult,
	4,
);

/// Argument for the OnInstanceSetup hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct OnInstanceSetupArg {
	/// The ID of the instance
	pub id: String,
	/// The side of the instance
	pub side: Option<Side>,
	/// Path to the instance's game dir
	pub game_dir: Option<String>,
	/// Version info for the instance
	pub version_info: VersionInfo,
	/// The loader of the instance
	pub loader: Loader,
	/// The current version of the loader, as stored in the lockfile. Can be used to detect version changes.
	pub current_loader_version: Option<String>,
	/// The desired version of the loader
	pub desired_loader_version: Option<VersionPattern>,
	/// Instance configuration
	pub config: InstanceConfig,
	/// Path to the Nitrolaunch internal dir
	pub internal_dir: String,
	/// The depth to update at
	pub update_depth: UpdateDepth,
	/// Path to the JVM
	pub jvm_path: String,
	/// Path to the vanilla game JAR
	pub game_jar_path: String,
}

/// Result from the OnInstanceSetup hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct OnInstanceSetupResult {
	/// Optional override for the main class
	pub main_class_override: Option<String>,
	/// Optional override for the path to the game JAR file
	pub jar_path_override: Option<String>,
	/// Optional extension to the classpath, as a list of paths
	pub classpath_extension: Vec<String>,
	/// Optional new version for the loader
	pub loader_version: Option<String>,
	/// Optional additional JVM args
	pub jvm_args: Vec<String>,
	/// Optional additional game args
	pub game_args: Vec<String>,
}

def_hook!(
	RemoveLoader,
	"remove_loader",
	"Hook for removing a loader from an instance when the loader or version changes",
	OnInstanceSetupArg,
	(),
	1,
);

def_hook!(
	AfterPackagesInstalled,
	"after_packages_installed",
	"Hook for doing work on an instance after packages are installed on that instance",
	AfterPackagesInstalledArg,
	(),
	2,
);

/// Argument for the AfterPackagesInstalled hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AfterPackagesInstalledArg {
	/// The ID of the instance
	pub id: String,
	/// The side of the instance
	pub side: Option<Side>,
	/// Path to the instance's game dir
	pub game_dir: Option<String>,
	/// Version info for the instance
	pub version_info: VersionInfo,
	/// The loader of the instance
	pub loader: Loader,
	/// Instance configuration
	pub config: InstanceConfig,
	/// Path to the Nitrolaunch internal dir
	pub internal_dir: String,
	/// The depth to update at
	pub update_depth: UpdateDepth,
}

def_hook!(
	OnInstanceLaunch,
	"on_instance_launch",
	"Hook for doing work before an instance is launched",
	InstanceLaunchArg,
	(),
	2,
);

def_hook!(
	WhileInstanceLaunch,
	"while_instance_launch",
	"Hook for running sibling processes with an instance when it is launched",
	InstanceLaunchArg,
	(),
	2,
	true,
);

def_hook!(
	OnInstanceStop,
	"on_instance_stop",
	"Hook for doing work when an instance is stopped gracefully",
	InstanceLaunchArg,
	(),
	2,
);

def_hook!(
	ReplaceInstanceLaunch,
	"replace_instance_launch",
	"Hook for replacing the launch behavior of an instance without a game dir",
	InstanceLaunchArg,
	Option<ReplaceInstanceLaunchResult>,
	1,
);

/// Result from the ReplaceInstanceLaunch hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ReplaceInstanceLaunchResult {
	/// The PID of the newly spawned instance process
	pub pid: u32,
}

def_hook!(
	UpdateWorldFiles,
	"update_world_files",
	"Hook for when shared world files are updated",
	InstanceLaunchArg,
	(),
	1,
);

/// Argument for the OnInstanceLaunch and WhileInstanceLaunch hooks
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InstanceLaunchArg {
	/// The ID of the instance
	pub id: String,
	/// The side of the instance
	pub side: Option<Side>,
	/// Path to the instance's dir
	pub dir: String,
	/// Path to the instance's game dir
	pub game_dir: Option<String>,
	/// Version info for the instance
	pub version_info: VersionInfo,
	/// The instance's configuration
	pub config: InstanceConfig,
	/// The PID of the instance process
	pub pid: Option<u32>,
	/// The path to the file containing the instance stdout and stderr. Will not be available in the on_instance_launch hook.
	pub stdout_path: Option<String>,
	/// The path to the file containing the instance stdin. Will not be available in the on_instance_launch hook.
	pub stdin_path: Option<String>,
}

def_hook!(
	CustomPackageInstruction,
	"custom_package_instruction",
	"Hook for handling custom instructions in packages",
	CustomPackageInstructionArg,
	CustomPackageInstructionResult,
	1,
	true,
);

/// Argument for the CustomPackageInstruction hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CustomPackageInstructionArg {
	/// The ID of the package
	pub pkg_id: String,
	/// The name of the custom command
	pub command: String,
	/// Any additional arguments supplied to the command
	pub args: Vec<String>,
}

/// Result from the CustomPackageInstruction hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CustomPackageInstructionResult {
	/// Whether the instruction was handled by this plugin
	pub handled: bool,
	/// The output of addon requests
	pub addon_reqs: Vec<AddonInstructionData>,
	/// The output dependencies
	pub deps: Vec<Vec<RequiredPackage>>,
	/// The output conflicts
	pub conflicts: Vec<PackageID>,
	/// The output recommendations
	pub recommendations: Vec<RecommendedPackage>,
	/// The output bundled packages
	pub bundled: Vec<PackageID>,
	/// The output compats
	pub compats: Vec<(PackageID, PackageID)>,
	/// The output package extensions
	pub extensions: Vec<PackageID>,
	/// The output notices
	pub notices: Vec<String>,
}

def_hook!(
	HandleAuth,
	"handle_auth",
	"Hook for handling authentication for custom user types",
	HandleAuthArg,
	HandleAuthResult,
	1,
);

/// Argument for the HandleAuth hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HandleAuthArg {
	/// The ID of the user
	pub user_id: String,
	/// The custom type of the user
	pub user_type: String,
}

/// Result from the HandleAuth hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HandleAuthResult {
	/// Whether the auth for this user type was handled by this plugin
	pub handled: bool,
	/// The resulting user profile
	pub profile: Option<MinecraftUserProfile>,
}

def_hook!(
	AddTranslations,
	"add_translations",
	"Hook for adding extra translations to Nitrolaunch",
	(),
	LanguageMap,
	1,
	true,
);

def_hook!(
	AddInstanceTransferFormats,
	"add_instance_transfer_formats",
	"Hook for adding information about instance transfer formats",
	(),
	Vec<InstanceTransferFormat>,
	1,
);

/// Information about an instance transfer format
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InstanceTransferFormat {
	/// The ID for this format
	pub id: String,
	/// A display name for this format
	pub name: String,
	/// A CSS color for this format
	pub color: Option<String>,
	/// Info for the import side of this format
	pub import: Option<InstanceTransferFormatDirection>,
	/// Info for the export side of this format
	pub export: Option<InstanceTransferFormatDirection>,
	/// Info for the migration side of this format
	pub migrate: Option<InstanceTransferFormatDirection>,
}

/// Information about a side of an instance transfer format
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InstanceTransferFormatDirection {
	/// Support status of the modloader
	pub modloader: InstanceTransferFeatureSupport,
	/// Support status of the mods
	pub mods: InstanceTransferFeatureSupport,
	/// Support status of the launch settings
	pub launch_settings: InstanceTransferFeatureSupport,
}

/// Support status of some feature in an instance transfer format
#[derive(Serialize, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum InstanceTransferFeatureSupport {
	/// This feature is supported by the transfer
	#[default]
	Supported,
	/// This feature is unsupported by the nature of the format
	FormatUnsupported,
	/// This feature is not yet supported by the plugin
	PluginUnsupported,
}

def_hook!(
	ExportInstance,
	"export_instance",
	"Hook for exporting an instance",
	ExportInstanceArg,
	(),
	3,
);

/// Argument provided to the export_instance hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ExportInstanceArg {
	/// The ID of the transfer format being used
	pub format: String,
	/// The ID of the instance
	pub id: String,
	/// The configuration of the instance
	pub config: InstanceConfig,
	/// The actual Minecraft version of the instance
	pub minecraft_version: String,
	/// The actual loader version of the instance
	pub loader_version: Option<String>,
	/// The directory where the instance game files are located
	pub game_dir: String,
	/// The desired path for the resulting instance, as a file path
	pub result_path: String,
}

def_hook!(
	ImportInstance,
	"import_instance",
	"Hook for importing an instance",
	ImportInstanceArg,
	ImportInstanceResult,
	2,
);

/// Argument provided to the import_instance hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ImportInstanceArg {
	/// The ID of the transfer format being used
	pub format: String,
	/// The ID of the new instance
	pub id: String,
	/// The path to the instance to import
	pub source_path: String,
	/// The desired directory for the resulting instance
	pub result_path: String,
}

/// Result from the ImportInstance hook giving information about the new instance
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ImportInstanceResult {
	/// The ID of the transfer format being used
	pub format: String,
	/// The configuration of the new instance
	pub config: InstanceConfig,
}

def_hook!(
	MigrateInstances,
	"migrate_instances",
	"Hook for migrating all instances from another launcher",
	String,
	MigrateInstancesResult,
	1,
);

/// Result from the MigrateInstances hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MigrateInstancesResult {
	/// The ID of the transfer format being used
	pub format: String,
	/// The configuration of the new instances
	pub instances: HashMap<String, InstanceConfig>,
	/// Map of instances to packages installed on the migrated instance
	pub packages: HashMap<String, Vec<MigratedPackage>>,
}

/// A package installed on a migrated instance
#[derive(Serialize, Deserialize)]
pub struct MigratedPackage {
	/// The ID of this package
	pub id: String,
	/// The addons currently installed with this package
	pub addons: Vec<MigratedAddon>,
}

/// An addon installed on a migrated instance
#[derive(Serialize, Deserialize)]
pub struct MigratedAddon {
	/// The unique ID of the addon in this package
	pub id: String,
	/// The paths to the addon
	pub paths: Vec<String>,
	/// What kind of addon this is
	pub kind: AddonKind,
	/// The currently installed addon version ID
	pub version: Option<String>,
}

def_hook!(
	AddSupportedLoaders,
	"add_supported_loaders",
	"Tell Nitrolaunch that you support installing extra loaders",
	(),
	Vec<Loader>,
	2,
);

def_hook!(
	AddInstances,
	"add_instances",
	"Hook for adding new instances",
	AddInstancesArg,
	HashMap<InstanceID, InstanceConfig>,
	1,
	true,
);

/// Argument for the AddInstances hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AddInstancesArg {}

def_hook!(
	AddTemplates,
	"add_templates",
	"Hook for adding new templates",
	AddInstancesArg,
	HashMap<TemplateID, TemplateConfig>,
	1,
	true,
);

def_hook!(
	InjectPageScript,
	"inject_page_script",
	"Hook for running JavaScript on GUI pages",
	InjectPageScriptArg,
	String,
	1,
	true,
);

/// Argument for the InjectPageScript hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InjectPageScriptArg {
	/// The identifier for the page
	pub page: String,
	/// The identifier for whatever 'thing' this page is representing. Could be an instance, template, anything else, or nothing.
	pub object: Option<String>,
}

def_hook!(
	AddSidebarButtons,
	"add_sidebar_buttons",
	"Hook for adding buttons to the GUI sidebar",
	(),
	Vec<SidebarButton>,
	1,
	true,
);

/// Data for a GUI sidebar button
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SidebarButton {
	/// The inner HTML of the button
	pub html: String,
	/// Where the button should go when pressed
	pub href: String,
	/// What the current URL should equal to select this item
	pub selected_url: Option<String>,
	/// What the current URL should start with to select this item
	pub selected_url_start: Option<String>,
	/// The CSS color of this button
	pub color: String,
}

def_hook!(
	GetPage,
	"get_page",
	"Hook for adding pages to the GUI",
	String,
	Option<String>,
	1,
);

def_hook!(
	AddCustomPackageRepositories,
	"add_custom_package_repositories",
	"Hook for adding custom package repositories",
	(),
	Vec<AddCustomPackageRepositoriesResult>,
	1,
);

/// A single added package repository from the AddCustomPackageRepositories hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AddCustomPackageRepositoriesResult {
	/// Whether the repository should be preferred or backup
	pub is_preferred: bool,
	/// The ID for the repository
	pub id: String,
	/// The metadata for the repository
	pub metadata: RepoMetadata,
}

def_hook!(
	QueryCustomPackageRepository,
	"query_custom_package_repository",
	"Hook for getting packages from a custom repository",
	QueryCustomPackageRepositoryArg,
	Option<CustomRepoQueryResult>,
	1,
);

/// Argument for the QueryCustomPackageRepository hook
#[derive(Serialize, Deserialize, Default)]
pub struct QueryCustomPackageRepositoryArg {
	/// The repository that is being queried
	pub repository: String,
	/// The package that is being asked for
	pub package: String,
}

/// Result from querying a custom package repository
#[derive(Serialize, Deserialize)]
pub struct CustomRepoQueryResult {
	/// The contents of the package
	pub contents: String,
	/// The content type of the package
	pub content_type: PackageContentType,
	/// The flags for the package
	pub flags: HashSet<PackageFlag>,
}

def_hook!(
	SearchCustomPackageRepository,
	"search_custom_package_repository",
	"Hook for searching or browsing a custom package repository",
	SearchCustomPackageRepositoryArg,
	PackageSearchResults,
	1,
);

/// Argument for the SearchCustomPackageRepository hook
#[derive(Serialize, Deserialize, Default)]
pub struct SearchCustomPackageRepositoryArg {
	/// The repository that is being queried
	pub repository: String,
	/// The parameters for the search
	pub parameters: PackageSearchParameters,
}

def_hook!(
	PreloadPackages,
	"preload_packages",
	"Hook for loading multiple packages from a custom package repository",
	PreloadPackagesArg,
	(),
	1,
);

/// Argument for the PreloadPackages hook
#[derive(Serialize, Deserialize, Default)]
pub struct PreloadPackagesArg {
	/// The repository that is being queried
	pub repository: String,
	/// The packages that are being preloaded
	pub packages: Vec<String>,
}

def_hook!(
	SyncCustomPackageRepository,
	"sync_custom_package_repository",
	"Hook for updating the cache of a custom package repository",
	SyncCustomPackageRepositoryArg,
	(),
	1,
);

/// Argument for the SyncCustomPackageRepository hook
#[derive(Serialize, Deserialize, Default)]
pub struct SyncCustomPackageRepositoryArg {
	/// The repository that is being synced
	pub repository: String,
}

def_hook!(
	AddThemes,
	"add_themes",
	"Hook for adding new GUI themes",
	(),
	Vec<Theme>,
	1,
	true,
);

/// Data for a GUI theme
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Theme {
	/// A unique ID for the theme
	pub id: String,
	/// The name of the theme
	pub name: String,
	/// A description for the theme
	pub description: Option<String>,
	/// The CSS data for the theme
	pub css: String,
	/// A css color that identifies this theme
	pub color: String,
}

def_hook!(
	CustomAction,
	"custom_action",
	"Runs an arbitrary action on this plugin",
	CustomActionArg,
	serde_json::Value,
	1,
);

/// Argument for the CustomAction hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CustomActionArg {
	/// The ID of the action
	pub id: String,
	/// The payload/argument for the action
	pub payload: serde_json::Value,
}

def_hook!(
	AddDropdownButtons,
	"add_dropdown_buttons",
	"Adds buttons to dropdowns in the GUI",
	(),
	Vec<DropdownButton>,
	1,
);

/// Button for GUI dropdowns
#[derive(Serialize, Deserialize)]
pub struct DropdownButton {
	/// The plugin which this button is from
	pub plugin: String,
	/// Which dropdown this button should be under
	pub location: DropdownButtonLocation,
	/// The icon for this button
	pub icon: String,
	/// The text for this button
	pub text: String,
	/// The CSS color for this button
	pub color: Option<String>,
	/// An optional tooltip for this button
	pub tip: Option<String>,
	/// The custom action to do when this button is clicked
	pub action: Option<String>,
	/// Javascript to run when this button is clicked
	pub on_click: Option<String>,
}

/// Location for a DropdownButton in the UI
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DropdownButtonLocation {
	/// Button on the instances page for adding an instance or template
	AddTemplateOrInstance,
	/// Button on an instance page for launching the instance
	InstanceLaunch,
	/// Button on an instance page for updating the instance
	InstanceUpdate,
	/// Button on an instance page for more options
	InstanceMoreOptions,
}

def_hook!(
	AddInstanceTiles,
	"add_instance_tiles",
	"Adds tiles to the instance page in the GUI",
	String,
	Vec<InstanceTile>,
	1,
	true,
);

/// Tile on the GUI instance page
#[derive(Serialize, Deserialize)]
pub struct InstanceTile {
	/// Unique ID for this tile
	pub id: String,
	/// HTML contents of this tile
	pub contents: String,
	/// The size of this tile
	pub size: InstanceTileSize,
}

/// Size of an InstanceTile
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum InstanceTileSize {
	/// Spans one unit
	Small,
	/// Spans two units
	Large,
}

def_hook!(
	AddInstanceIcons,
	"add_instance_icons",
	"Adds default instance icons for the user to use",
	(),
	Vec<String>,
	1,
	true,
);

def_hook!(
	GetLoaderVersions,
	"get_loader_versions",
	"Gets the list of versions for a loader",
	GetLoaderVersionsArg,
	Vec<String>,
	1,
	true,
);

/// Argument for the GetLoaderVersions hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GetLoaderVersionsArg {
	/// The loader to get versions for
	pub loader: Loader,
	/// The Minecraft version of the instance
	pub minecraft_version: String,
}

def_hook!(
	AddUserTypes,
	"add_user_types",
	"Adds new available user types",
	(),
	Vec<UserTypeInfo>,
	1,
);

/// Information about a user type
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UserTypeInfo {
	/// The ID of the user type
	pub id: String,
	/// The display name of the user type
	pub name: String,
	/// CSS color of the user type
	pub color: String,
}

def_hook!(
	AddJavaTypes,
	"get_java_types",
	"Adds new available Java types",
	(),
	Vec<JavaTypeInfo>,
	1,
);

/// Information about a Java type
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct JavaTypeInfo {
	/// The ID of the Java type
	pub id: String,
	/// The display name of the Java type
	pub name: String,
	/// CSS color of the Java type
	pub color: String,
}

def_hook!(
	InstallCustomJava,
	"install_custom_java",
	"Installs a custom Java type",
	InstallCustomJavaArg,
	Option<InstallCustomJavaResult>,
	1,
	true,
);

/// Argument for the InstallCustomJava hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InstallCustomJavaArg {
	/// The Java that is being installed
	pub kind: String,
	/// The major version of Java to install
	pub major_version: String,
	/// Update depth for the install
	pub update_depth: UpdateDepth,
}

/// Result from the InstallCustomJava hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InstallCustomJavaResult {
	/// The path to the Java installation, containing a bin/java(.exe) executable
	pub path: String,
	/// The version of the resulting installation
	pub version: String,
}
