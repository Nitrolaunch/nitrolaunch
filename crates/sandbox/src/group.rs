use std::path::Path;

use nitro_shared::Side;
use nitro_shared::io::home_dir;
use serde::{Deserialize, Serialize};

use crate::policy::{FilesystemPolicy, ResolvedSandboxPolicy};

/// Default, no-brainer policy groups for the sandbox, necessary for the instance to function
pub static DEFAULT_POLICY_GROUPS: &[PolicyGroup] = &[
	PolicyGroup::Base,
	PolicyGroup::Instance,
	PolicyGroup::GameFiles,
	PolicyGroup::Devices,
];

/// Standard presets of policies for the sandbox
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PolicyGroup {
	/// Allows access to launching Java, stdin/out, and system files
	Base,
	/// Allows access to shared game files, such as libraries and assets
	GameFiles,
	/// Allows access to the instance's files
	Instance,
	/// Allows access to all devices
	Devices,
	/// Allows access to the network
	Network,
	/// Allows access to input devices like keyboards and mice
	InputDevices,
	/// Allows access to graphics devices like GPUs
	GraphicsDevices,
}

impl PolicyGroup {
	pub(crate) fn resolve(
		&self,
		params: &GroupResolveParams,
		resolved: &mut ResolvedSandboxPolicy,
	) {
		match self {
			Self::Base => {
				resolved.allowed_paths.insert(
					params.java_installation.to_string_lossy().into(),
					FilesystemPolicy::ReadWrite,
				);
				resolved.allowed_paths.insert(
					params
						.java_installation
						.join("bin/java")
						.to_string_lossy()
						.into(),
					FilesystemPolicy::Execute,
				);
				if let Some(stdout_file) = params.stdout_file {
					resolved.allowed_paths.insert(
						stdout_file.to_string_lossy().into(),
						FilesystemPolicy::ReadWrite,
					);
				}
				if let Some(stdin_file) = params.stdin_file {
					resolved.allowed_paths.insert(
						stdin_file.to_string_lossy().into(),
						FilesystemPolicy::ReadWrite,
					);
				}

				#[cfg(target_os = "linux")]
				{
					let read_paths = [
						"/usr/lib",
						"/usr/lib64",
						"/lib",
						"/lib64",
						"/etc/pki",
						"/etc/ssl",
						"/etc",
					];
					let write_paths = ["/proc/self", "/tmp"];
					for path in read_paths {
						resolved
							.allowed_paths
							.insert(path.into(), FilesystemPolicy::Read);
					}
					for path in write_paths {
						resolved
							.allowed_paths
							.insert(path.into(), FilesystemPolicy::ReadWrite);
					}
				}
			}
			Self::GameFiles => {
				for dir in [
					params.jars_dir,
					params.assets_dir,
					params.libraries_dir,
					params.natives_dir,
					params.versions_dir,
				] {
					resolved
						.allowed_paths
						.insert(dir.to_string_lossy().into(), FilesystemPolicy::Read);
				}
			}
			Self::Instance => {
				resolved.allowed_paths.insert(
					params.instance_dir.to_string_lossy().into(),
					FilesystemPolicy::ReadWrite,
				);
			}
			Self::Devices => {
				#[cfg(target_os = "linux")]
				{
					resolved
						.allowed_paths
						.insert("/dev".into(), FilesystemPolicy::ReadWrite);
				}
			}
			Self::GraphicsDevices => {
				#[cfg(target_os = "linux")]
				{
					let write_paths = [
						"/dev/dri",
						"/dev/nvidia0",
						"/dev/nvidiactl",
						"/dev/shm",
						"/sys/class/drm",
						"/tmp/.X11-unix",
						"/usr/share/vulkan",
						"/usr/share/glvnd",
						"/usr/share/X11/xorg.conf.d",
					];
					for path in write_paths {
						resolved
							.allowed_paths
							.insert(path.into(), FilesystemPolicy::ReadWrite);
					}
					let read_paths = ["/proc/driver/nvidia"];
					for path in read_paths {
						resolved
							.allowed_paths
							.insert(path.into(), FilesystemPolicy::Read);
					}

					if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
						resolved
							.allowed_paths
							.insert(runtime_dir.into(), FilesystemPolicy::ReadWrite);
					}

					let x_authority = std::env::var("XAUTHORITY").unwrap_or_else(|_| {
						let home = home_dir().unwrap_or("/home".into());
						home.join(".Xauthority").to_string_lossy().into()
					});
					resolved
						.allowed_paths
						.insert(x_authority.into(), FilesystemPolicy::ReadWrite);

					for path in glob::glob("/sys/devices/pci*").into_iter().flatten() {
						if let Ok(path) = path {
							resolved
								.allowed_paths
								.insert(path.to_string_lossy().into(), FilesystemPolicy::ReadWrite);
						}
					}
				}
			}
			Self::InputDevices => {}
			Self::Network => {
				resolved.allowed_hosts.push("localhost".into());
				resolved.allowed_ports.push(25565);
				resolved.allowed_ports.push(80);
				resolved.allowed_ports.push(443);
			}
		}
	}
}

/// Parameters for resolving policy groups into their individual rules
#[allow(missing_docs)]
pub struct GroupResolveParams<'a> {
	pub side: Side,
	pub instance_dir: &'a Path,
	pub java_installation: &'a Path,
	pub jars_dir: &'a Path,
	pub assets_dir: &'a Path,
	pub libraries_dir: &'a Path,
	pub natives_dir: &'a Path,
	pub versions_dir: &'a Path,
	pub stdout_file: Option<&'a Path>,
	pub stdin_file: Option<&'a Path>,
}
