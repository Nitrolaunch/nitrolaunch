/// Version of the API for executable plugins
#[cfg(feature = "executable_api")]
pub mod executable;
/// Version of the API for WASM plugins
#[cfg(any(feature = "wasm_api", target_family = "wasm"))]
pub mod wasm;

#[cfg(any(feature = "executable_api", feature = "wasm_api"))]
use crate::hook::Hook;
#[cfg(feature = "executable_api")]
use executable::{ExecutablePlugin, HookContext};
#[cfg(any(feature = "wasm_api", target_family = "wasm"))]
use wasm::WASMPlugin;

macro_rules! hook_interface {
	($name:ident, $name2:literal, $hook:ident, $arg:expr) => {
		#[cfg(feature = "executable_api")]
		impl ExecutablePlugin {
			#[doc = concat!("Bind to the ", $name2, " hook")]
			pub fn $name(
				&mut self,
				f: impl FnOnce(
					HookContext<$crate::hook::hooks::$hook>,
					<$crate::hook::hooks::$hook as Hook>::Arg,
				) -> anyhow::Result<<$crate::hook::hooks::$hook as Hook>::Result>,
			) -> anyhow::Result<()> {
				self.handle_hook::<$crate::hook::hooks::$hook>($arg, f)
			}
		}

		#[cfg(any(feature = "wasm_api", target_family = "wasm"))]
		impl WASMPlugin {
			#[doc = concat!("Bind to the ", $name2, " hook")]
			pub fn $name(
				&mut self,
				f: impl FnOnce(
					<$crate::hook::hooks::$hook as Hook>::Arg,
				) -> anyhow::Result<<$crate::hook::hooks::$hook as Hook>::Result>,
			) -> anyhow::Result<()> {
				self.handle_hook::<$crate::hook::hooks::$hook>($arg, f)
			}
		}
	};

	($name:ident, $name2:literal, $hook:ident) => {
		hook_interface!($name, $name2, $hook, Self::get_hook_arg);
	};
}

hook_interface!(on_load, "on_load", OnLoad, |_| Ok(()));
hook_interface!(start_worker, "start_worker", StartWorker, |_| Ok(()));
hook_interface!(subcommand, "subcommand", Subcommand);
hook_interface!(
	modify_instance_config,
	"modify_instance_config",
	ModifyInstanceConfig
);
hook_interface!(add_versions, "add_versions", AddVersions);
hook_interface!(on_instance_setup, "on_instance_setup", OnInstanceSetup);
hook_interface!(on_instance_launch, "on_instance_launch", OnInstanceLaunch);
hook_interface!(
	while_instance_launch,
	"while_instance_launch",
	WhileInstanceLaunch
);
hook_interface!(on_instance_stop, "on_instance_stop", OnInstanceStop);
hook_interface!(update_world_files, "update_world_files", UpdateWorldFiles);
hook_interface!(
	custom_package_instruction,
	"custom_package_instruction",
	CustomPackageInstruction
);
hook_interface!(handle_auth, "handle_auth", HandleAuth);
hook_interface!(add_translations, "add_translations", AddTranslations);
hook_interface!(
	add_instance_transfer_formats,
	"add_instance_transfer_formats",
	AddInstanceTransferFormats
);
hook_interface!(export_instance, "export_instance", ExportInstance);
hook_interface!(import_instance, "import_instance", ImportInstance);
hook_interface!(migrate_instances, "migrate_instances", MigrateInstances);
hook_interface!(
	add_supported_loaders,
	"add_supported_loaders",
	AddSupportedLoaders
);
hook_interface!(remove_loader, "remove_loader", RemoveLoader);
hook_interface!(add_instances, "add_instances", AddInstances);
hook_interface!(add_templates, "add_templates", AddTemplates);
hook_interface!(inject_page_script, "inject_page_script", InjectPageScript);
hook_interface!(
	add_sidebar_buttons,
	"add_sidebar_buttons",
	AddSidebarButtons
);
hook_interface!(get_page, "get_page", GetPage);
hook_interface!(
	add_custom_package_repositories,
	"add_custom_package_repositories",
	AddCustomPackageRepositories
);
hook_interface!(
	query_custom_package_repository,
	"query_custom_package_repository",
	QueryCustomPackageRepository
);
hook_interface!(
	search_custom_package_repository,
	"search_custom_package_repository",
	SearchCustomPackageRepository
);
hook_interface!(preload_packages, "preload_packages", PreloadPackages);
hook_interface!(
	sync_custom_package_repository,
	"sync_custom_package_repository",
	SyncCustomPackageRepository
);
hook_interface!(add_themes, "add_themes", AddThemes);
hook_interface!(custom_action, "custom_action", CustomAction);
hook_interface!(
	add_dropdown_buttons,
	"add_dropdown_buttons",
	AddDropdownButtons
);
hook_interface!(add_instance_tiles, "add_instance_tiles", AddInstanceTiles);
hook_interface!(
	after_packages_installed,
	"after_packages_installed",
	AfterPackagesInstalled
);
hook_interface!(add_instance_icons, "add_instance_icons", AddInstanceIcons);
hook_interface!(
	get_loader_versions,
	"get_loader_versions",
	GetLoaderVersions
);
hook_interface!(add_user_types, "add_user_types", AddUserTypes);
hook_interface!(add_java_types, "add_java_types", AddJavaTypes);
hook_interface!(
	install_custom_java,
	"install_custom_java",
	InstallCustomJava
);
