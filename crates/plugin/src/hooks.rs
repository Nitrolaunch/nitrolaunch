use std::collections::{HashMap, HashSet};

use mcvm_config::instance::InstanceConfig;
use mcvm_config::profile::ProfileConfig;
use mcvm_core::net::game_files::version_manifest::VersionEntry;
use mcvm_core::net::minecraft::MinecraftUserProfile;
use mcvm_pkg::repo::{PackageFlag, RepoMetadata};
use mcvm_pkg::script_eval::AddonInstructionData;
use mcvm_pkg::{PackageContentType, PackageSearchResults, RecommendedPackage, RequiredPackage};
use mcvm_shared::id::{InstanceID, ProfileID};
use mcvm_shared::lang::translate::LanguageMap;
use mcvm_shared::loaders::Loader;
use mcvm_shared::pkg::{PackageID, PackageSearchParameters};
use mcvm_shared::versions::VersionPattern;
use mcvm_shared::UpdateDepth;
use mcvm_shared::{output::MCVMOutput, versions::VersionInfo, Side};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::hook_call::HookCallArg;
use crate::HookHandle;

/// Trait for a hook that can be called
pub trait Hook {
	/// The type for the argument that goes into the hook
	type Arg: Serialize + DeserializeOwned;
	/// The type for the result from the hook
	type Result: DeserializeOwned + Serialize + Default;

	/// Get the name of the hook
	fn get_name(&self) -> &'static str {
		Self::get_name_static()
	}

	/// Get the name of the hook statically
	fn get_name_static() -> &'static str;

	/// Get whether the hook should forward all output to the terminal
	fn get_takes_over() -> bool {
		false
	}

	/// Get the version number of the hook
	fn get_version() -> u16;

	/// Call the hook using the specified program
	#[allow(async_fn_in_trait)]
	async fn call(
		&self,
		arg: HookCallArg<'_, Self>,
		o: &mut impl MCVMOutput,
	) -> anyhow::Result<HookHandle<Self>>
	where
		Self: Sized,
	{
		crate::hook_call::call(self, arg, o).await
	}
}

macro_rules! def_hook {
	($struct:ident, $name:literal, $desc:literal, $arg:ty, $res:ty, $version:literal, $($extra:tt)*) => {
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

			$(
				$extra
			)*
		}
	};
}

def_hook!(
	OnLoad,
	"on_load",
	"Hook for when a plugin is loaded",
	(),
	(),
	1,
);

def_hook!(
	StartWorker,
	"start_worker",
	"Hook for starting a long-running worker alongside the plugin runner",
	(),
	(),
	1,
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
	(),
	Vec<VersionEntry>,
	1,
);

def_hook!(
	OnInstanceSetup,
	"on_instance_setup",
	"Hook for doing work when setting up an instance for update or launch",
	OnInstanceSetupArg,
	OnInstanceSetupResult,
	3,
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
	pub game_dir: String,
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
	/// Path to the MCVM internal dir
	pub internal_dir: String,
	/// The depth to update at
	pub update_depth: UpdateDepth,
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
);

def_hook!(
	OnInstanceStop,
	"on_instance_stop",
	"Hook for doing work when an instance is stopped gracefully",
	InstanceLaunchArg,
	(),
	2,
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
	pub game_dir: String,
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
	"Hook for adding extra translations to MCVM",
	(),
	LanguageMap,
	1,
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
	/// Info for the import side of this format
	pub import: Option<InstanceTransferFormatDirection>,
	/// Info for the export side of this format
	pub export: Option<InstanceTransferFormatDirection>,
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
	AddSupportedLoaders,
	"add_supported_loaders",
	"Tell MCVM that you support installing extra loaders",
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
);

/// Argument for the AddInstances hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AddInstancesArg {}

def_hook!(
	AddProfiles,
	"add_profiles",
	"Hook for adding new profiles",
	AddInstancesArg,
	HashMap<ProfileID, ProfileConfig>,
	1,
);

def_hook!(
	InjectPageScript,
	"inject_page_script",
	"Hook for running JavaScript on GUI pages",
	InjectPageScriptArg,
	String,
	1,
);

/// Argument for the InjectPageScript hook
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InjectPageScriptArg {
	/// The identifier for the page
	pub page: String,
	/// The identifier for whatever 'thing' this page is representing. Could be an instance, profile, anything else, or nothing.
	pub object: Option<String>,
}

def_hook!(
	AddSidebarButtons,
	"add_sidebar_buttons",
	"Hook for adding buttons to the GUI sidebar",
	(),
	Vec<SidebarButton>,
	1,
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
