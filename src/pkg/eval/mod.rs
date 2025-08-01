/// Evaluating script package conditions
pub mod conditions;
/// Evaluating declarative packages
pub mod declarative;
/// Evaluating script packages
pub mod script;

use anyhow::bail;
use anyhow::Context;
use async_trait::async_trait;
use nitro_config::package::EvalPermissions;
use nitro_parse::routine::INSTALL_ROUTINE;
use nitro_parse::vars::HashMapVariableStore;
use nitro_pkg::overrides::PackageOverrides;
use nitro_pkg::properties::PackageProperties;
use nitro_pkg::resolve::ResolutionResult;
use nitro_pkg::script_eval::AddonInstructionData;
use nitro_pkg::script_eval::EvalReason;
use nitro_pkg::ConfiguredPackage;
use nitro_pkg::PackageContentType;
use nitro_pkg::RecommendedPackage;
use nitro_pkg::RequiredPackage;
use nitro_pkg::{
	EvalInput as EvalInputTrait, PackageEvalRelationsResult as EvalRelationsResultTrait,
	PackageEvaluator as PackageEvaluatorTrait,
};
use nitro_shared::addon::{is_addon_version_valid, is_filename_valid, Addon};
use nitro_shared::lang::Language;
use nitro_shared::loaders::Loader;
use nitro_shared::output;
use nitro_shared::output::MessageContents;
use nitro_shared::output::MessageLevel;
use nitro_shared::output::NitroOutput;
use nitro_shared::output::Simple;
use nitro_shared::pkg::ArcPkgReq;
use nitro_shared::pkg::PackageID;
use nitro_shared::util::is_valid_identifier;
use reqwest::Client;

use self::conditions::check_arch_condition;
use self::conditions::check_os_condition;
use self::declarative::eval_declarative_package;
use self::script::eval_script_package;

use super::reg::PkgRegistry;
use super::Package;
use crate::addon::{self, AddonLocation, AddonRequest};
use crate::config::package::PackageConfig;
use crate::io::paths::Paths;
use crate::plugin::PluginManager;
use crate::util::hash::{
	get_hash_str_as_hex, HASH_SHA256_RESULT_LENGTH, HASH_SHA512_RESULT_LENGTH,
};
use nitro_shared::pkg::PackageStability;
use nitro_shared::Side;

use std::path::PathBuf;

/// Max notice instructions per package
const MAX_NOTICE_INSTRUCTIONS: usize = 10;
/// Max characters per notice instruction
const MAX_NOTICE_CHARACTERS: usize = 128;

/// Context / purpose for when we are evaluating
pub enum Routine {
	/// Install the package
	Install,
	/// Install routine, except for resolution
	InstallResolve,
}

impl Routine {
	/// Get the routine name of this routine
	pub fn get_routine_name(&self) -> String {
		match self {
			Self::Install => INSTALL_ROUTINE,
			Self::InstallResolve => INSTALL_ROUTINE,
		}
		.into()
	}

	/// Get the EvalReason of this routine
	pub fn get_reason(&self) -> EvalReason {
		match self {
			Self::Install => EvalReason::Install,
			Self::InstallResolve => EvalReason::Resolve,
		}
	}
}

/// Combination of both EvalConstants and EvalParameters
#[derive(Debug, Clone)]
pub struct EvalInput<'a> {
	/// Constant values
	pub constants: &'a EvalConstants,
	/// Changing values
	pub params: EvalParameters,
}

impl EvalInputTrait for EvalInput<'_> {
	fn set_content_versions(
		&mut self,
		required_versions: Vec<String>,
		preferred_versions: Vec<String>,
	) {
		self.params.required_content_versions = required_versions;
		self.params.preferred_content_versions = preferred_versions;
	}
}

/// Constants for the evaluation that will be the same across every package
#[derive(Debug, Clone)]
pub struct EvalConstants {
	/// The Minecraft version
	pub version: String,
	/// The loader used
	pub loader: Loader,
	/// The list of available Minecraft versions
	pub version_list: Vec<String>,
	/// The user's configured language
	pub language: Language,
	/// The configured default stability for the profile
	pub profile_stability: PackageStability,
}

/// Constants for the evaluation that may be different for each package
#[derive(Debug, Clone)]
pub struct EvalParameters {
	/// The side (client/server) we are installing the package on
	pub side: Side,
	/// Features enabled for the package
	pub features: Vec<String>,
	/// Permissions for the package
	pub perms: EvalPermissions,
	/// Requested stability of the package's contents
	pub stability: PackageStability,
	/// Requested worlds to put addons in
	pub worlds: Vec<String>,
	/// Required content versions for this package
	pub required_content_versions: Vec<String>,
	/// Preferred content versions for this package
	pub preferred_content_versions: Vec<String>,
}

impl EvalParameters {
	/// Create new EvalParameters with default parameters and a side
	pub fn new(side: Side) -> Self {
		Self {
			side,
			features: Vec::new(),
			perms: EvalPermissions::default(),
			stability: PackageStability::default(),
			worlds: Vec::new(),
			required_content_versions: Vec::new(),
			preferred_content_versions: Vec::new(),
		}
	}

	/// Apply a package config to the parameters
	pub fn apply_config(
		&mut self,
		config: &PackageConfig,
		properties: &PackageProperties,
	) -> anyhow::Result<()> {
		// Calculate features
		let features = config
			.calculate_features(properties)
			.context("Failed to calculate features")?;

		self.features = features;
		self.perms = config.permissions;
		self.stability = config.stability;

		Ok(())
	}
}

/// Persistent state for evaluation
#[derive(Clone)]
pub struct EvalData<'a> {
	/// Input to the evaluator
	pub input: EvalInput<'a>,
	/// Plugins
	pub plugins: PluginManager,
	/// ID of the package we are evaluating
	pub id: PackageID,
	/// Level of evaluation
	pub reason: EvalReason,
	/// Package properties
	pub properties: PackageProperties,
	/// Variables, used for script evaluation
	pub vars: HashMapVariableStore,
	/// The output of addon requests
	pub addon_reqs: Vec<AddonRequest>,
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
	/// The output commands
	pub commands: Vec<Vec<String>>,
	/// Whether the package uses custom instructions
	pub uses_custom_instructions: bool,
}

impl<'a> EvalData<'a> {
	/// Create a new EvalData
	pub fn new(
		input: EvalInput<'a>,
		id: PackageID,
		properties: PackageProperties,
		routine: &Routine,
		plugins: PluginManager,
	) -> Self {
		Self {
			input,
			id,
			plugins,
			reason: routine.get_reason(),
			properties,
			vars: HashMapVariableStore::default(),
			addon_reqs: Vec::new(),
			deps: Vec::new(),
			conflicts: Vec::new(),
			recommendations: Vec::new(),
			bundled: Vec::new(),
			compats: Vec::new(),
			extensions: Vec::new(),
			notices: Vec::new(),
			commands: Vec::new(),
			uses_custom_instructions: false,
		}
	}
}

impl Package {
	/// Evaluate a routine on a package
	pub async fn eval<'a>(
		&mut self,
		paths: &'a Paths,
		routine: Routine,
		input: EvalInput<'a>,
		client: &Client,
		plugins: PluginManager,
	) -> anyhow::Result<EvalData<'a>> {
		self.parse(paths, client).await?;

		// Check properties
		let properties = self.get_properties(paths, client).await?.clone();
		if eval_check_properties(&input, &properties)? {
			return Ok(EvalData::new(
				input,
				self.id.clone(),
				properties,
				&routine,
				plugins,
			));
		}

		match self.content_type {
			PackageContentType::Script => {
				let parsed = self.data.get_mut().contents.get_mut().get_script_contents();
				let eval = eval_script_package(
					self.id.clone(),
					parsed,
					routine,
					properties,
					input,
					plugins,
					paths,
				)
				.await?;
				Ok(eval)
			}
			PackageContentType::Declarative => {
				let contents = self.data.get().contents.get().get_declarative_contents();
				let eval = eval_declarative_package(
					self.id.clone(),
					contents,
					input,
					properties,
					routine,
					plugins,
				)?;
				Ok(eval)
			}
		}
	}
}

/// Check properties when evaluating. Returns true if the package should finish evaluating with no error
pub fn eval_check_properties(
	input: &EvalInput,
	properties: &PackageProperties,
) -> anyhow::Result<bool> {
	if let Some(supported_versions) = &properties.supported_versions {
		if !supported_versions.is_empty()
			&& !supported_versions
				.iter()
				.any(|x| x.matches_single(&input.constants.version, &input.constants.version_list))
		{
			bail!("Package does not support this Minecraft version");
		}
	}

	if let Some(supported_loaders) = &properties.supported_loaders {
		if !supported_loaders.is_empty()
			&& !supported_loaders
				.iter()
				.any(|x| x.matches(&input.constants.loader))
		{
			bail!("Package does not support this loader");
		}
	}

	if let Some(supported_sides) = &properties.supported_sides {
		if !supported_sides.is_empty() && !supported_sides.contains(&input.params.side) {
			return Ok(true);
		}
	}

	if let Some(supported_operating_systems) = &properties.supported_operating_systems {
		if !supported_operating_systems.is_empty()
			&& !supported_operating_systems.iter().any(check_os_condition)
		{
			bail!("Package does not support your operating system");
		}
	}

	if let Some(supported_architectures) = &properties.supported_architectures {
		if !supported_architectures.is_empty()
			&& !supported_architectures.iter().any(check_arch_condition)
		{
			bail!("Package does not support your system architecture");
		}
	}

	Ok(false)
}

/// Utility for evaluation that validates addon arguments and creates a request
pub fn create_valid_addon_request(
	data: AddonInstructionData,
	pkg_id: PackageID,
	eval_input: &EvalInput,
) -> anyhow::Result<AddonRequest> {
	if !is_valid_identifier(&data.id) {
		bail!("Invalid addon identifier '{}'", data.id);
	}

	// Empty strings will break the filename so we convert them to none
	let version = data.version.filter(|x| !x.is_empty());
	if let Some(version) = &version {
		if !is_addon_version_valid(version) {
			bail!(
				"Invalid addon version identifier '{version}' for addon '{}'",
				data.id
			);
		}
	}

	let file_name = data.file_name.unwrap_or(addon::get_addon_instance_filename(
		&pkg_id, &data.id, &data.kind,
	));

	if !is_filename_valid(data.kind, &file_name) {
		bail!(
			"Invalid addon filename '{file_name}' in addon '{}'",
			data.id
		);
	}

	// Check hashes
	if let Some(hash) = &data.hashes.sha256 {
		let hex = get_hash_str_as_hex(hash).context("Failed to parse hash string")?;
		if hex.len() > HASH_SHA256_RESULT_LENGTH {
			bail!(
				"SHA-256 hash for addon '{}' is longer than {HASH_SHA256_RESULT_LENGTH} characters",
				data.id
			);
		}
	}

	if let Some(hash) = &data.hashes.sha512 {
		let hex = get_hash_str_as_hex(hash).context("Failed to parse hash string")?;
		if hex.len() > HASH_SHA512_RESULT_LENGTH {
			bail!(
				"SHA-512 hash for addon '{}' is longer than {HASH_SHA512_RESULT_LENGTH} characters",
				data.id
			);
		}
	}

	let addon = Addon {
		kind: data.kind,
		id: data.id.clone(),
		file_name,
		pkg_id,
		version,
		hashes: data.hashes,
	};

	if let Some(url) = data.url {
		let location = AddonLocation::Remote(url);
		Ok(AddonRequest::new(addon, location))
	} else if let Some(path) = data.path {
		match eval_input.params.perms {
			EvalPermissions::Elevated => {
				let path = shellexpand::tilde(&path).to_string();
				let path = PathBuf::from(path);
				let location = AddonLocation::Local(path);
				Ok(AddonRequest::new(addon, location))
			}
			_ => {
				bail!(
					"Insufficient permissions to add a local addon '{}'",
					data.id
				);
			}
		}
	} else {
		bail!(
			"No location (url/path) was specified for addon '{}'",
			data.id
		);
	}
}

/// Evaluator used as an input for dependency resolution
struct PackageEvaluator<'a> {
	reg: &'a mut PkgRegistry,
}

/// Common argument for the evaluator
struct EvaluatorCommonInput<'a> {
	paths: &'a Paths,
	client: &'a Client,
}

/// Newtype for PkgInstanceConfig
#[derive(Clone)]
struct EvalPackageConfig(PackageConfig, ArcPkgReq);

impl ConfiguredPackage for EvalPackageConfig {
	type EvalInput<'a> = EvalInput<'a>;

	fn get_package(&self) -> ArcPkgReq {
		self.1.clone()
	}

	fn override_configured_package_input(
		&self,
		properties: &PackageProperties,
		input: &mut Self::EvalInput<'_>,
	) -> anyhow::Result<()> {
		input
			.params
			.apply_config(&self.0, properties)
			.context("Failed to apply config to parameters")?;

		Ok(())
	}
}

struct EvalRelationsResult {
	pub deps: Vec<Vec<RequiredPackage>>,
	pub conflicts: Vec<PackageID>,
	pub recommendations: Vec<nitro_pkg::RecommendedPackage>,
	pub bundled: Vec<PackageID>,
	pub compats: Vec<(PackageID, PackageID)>,
	pub extensions: Vec<PackageID>,
}

impl EvalRelationsResultTrait for EvalRelationsResult {
	fn get_bundled(&self) -> Vec<PackageID> {
		self.bundled.clone()
	}

	fn get_compats(&self) -> Vec<(PackageID, PackageID)> {
		self.compats.clone()
	}

	fn get_conflicts(&self) -> Vec<PackageID> {
		self.conflicts.clone()
	}

	fn get_deps(&self) -> Vec<Vec<RequiredPackage>> {
		self.deps.clone()
	}
	fn get_extensions(&self) -> Vec<PackageID> {
		self.extensions.clone()
	}

	fn get_recommendations(&self) -> Vec<nitro_pkg::RecommendedPackage> {
		self.recommendations.clone()
	}
}

#[async_trait]
impl<'a> PackageEvaluatorTrait<'a> for PackageEvaluator<'a> {
	type CommonInput = EvaluatorCommonInput<'a>;
	type ConfiguredPackage = EvalPackageConfig;
	type EvalInput<'b> = EvalInput<'b>;
	type EvalRelationsResult<'b> = EvalRelationsResult;

	async fn eval_package_relations(
		&mut self,
		pkg: &ArcPkgReq,
		input: &Self::EvalInput<'a>,
		common_input: &Self::CommonInput,
	) -> anyhow::Result<Self::EvalRelationsResult<'a>> {
		let eval = self
			.reg
			.eval(
				pkg,
				common_input.paths,
				Routine::InstallResolve,
				input.clone(),
				common_input.client,
				&mut output::NoOp,
			)
			.await
			.context("Failed to evaluate package")?;

		let result = EvalRelationsResult {
			deps: eval.deps,
			conflicts: eval.conflicts,
			recommendations: eval.recommendations,
			bundled: eval.bundled,
			compats: eval.compats,
			extensions: eval.extensions,
		};

		Ok(result)
	}

	async fn get_package_properties<'b>(
		&'b mut self,
		pkg: &ArcPkgReq,
		common_input: &Self::CommonInput,
	) -> anyhow::Result<&'b PackageProperties> {
		let properties = self
			.reg
			.get_properties(
				pkg,
				common_input.paths,
				common_input.client,
				&mut output::NoOp,
			)
			.await?;
		Ok(properties)
	}

	async fn preload_packages<'b>(
		&'b mut self,
		packages: &[ArcPkgReq],
		common_input: &Self::CommonInput,
	) -> anyhow::Result<()> {
		self.reg
			.preload_packages(
				packages.iter(),
				common_input.paths,
				common_input.client,
				&mut Simple(MessageLevel::Important),
			)
			.await
	}
}

/// Resolve package dependencies
#[allow(clippy::too_many_arguments)]
pub async fn resolve(
	packages: &[PackageConfig],
	instance_id: &str,
	constants: &EvalConstants,
	default_params: EvalParameters,
	overrides: PackageOverrides,
	paths: &Paths,
	reg: &mut PkgRegistry,
	client: &Client,
	o: &mut impl NitroOutput,
) -> anyhow::Result<ResolutionResult> {
	let evaluator = PackageEvaluator { reg };

	let input = EvalInput {
		constants,
		params: default_params,
	};

	let common_input = EvaluatorCommonInput { client, paths };

	let packages = packages
		.iter()
		.map(|x| EvalPackageConfig((*x).clone(), x.get_request()))
		.collect::<Vec<_>>();

	let result =
		match nitro_pkg::resolve::resolve(&packages, evaluator, input, &common_input, overrides)
			.await
		{
			Ok(result) => result,
			Err(e) => {
				o.display_special_resolution_error(e, instance_id);
				bail!("Package resolution failed");
			}
		};

	for package in &result.unfulfilled_recommendations {
		print_recommendation_warning(package, o);
	}

	Ok(result)
}

/// Prints an unfulfilled recommendation warning
fn print_recommendation_warning(
	package: &nitro_pkg::resolve::RecommendedPackage,
	o: &mut impl NitroOutput,
) {
	let source = package.req.source.get_source();
	let message = if package.invert {
		if let Some(source) = source {
			MessageContents::Warning(format!("The package '{}' recommends against the use of the package '{}', which is installed", source.debug_sources(), package.req))
		} else {
			MessageContents::Warning(format!(
				"A package recommends against the use of the package '{}', which is installed",
				package.req
			))
		}
	} else if let Some(source) = source {
		MessageContents::Warning(format!(
			"The package '{}' recommends the use of the package '{}', which is not installed",
			source.debug_sources(),
			package.req
		))
	} else {
		MessageContents::Warning(format!(
			"A package recommends the use of the package '{}', which is not installed",
			package.req
		))
	};

	o.display(message, MessageLevel::Important);
}
