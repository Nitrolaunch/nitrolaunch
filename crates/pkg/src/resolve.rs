use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use itertools::Itertools;
use nitro_shared::pkg::{ArcPkgReq, PackageID, ResolutionError};
use nitro_shared::versions::VersionPattern;

use crate::overrides::{is_package_overridden, PackageOverrides};
use crate::properties::PackageProperties;
use crate::{ConfiguredPackage, EvalInput, PackageEvalRelationsResult, PackageEvaluator};

use crate::{PkgRequest, PkgRequestSource};

/// Find all package dependencies from a set of required packages
pub async fn resolve<'a, E: PackageEvaluator<'a>>(
	packages: &[E::ConfiguredPackage],
	mut evaluator: E,
	constant_eval_input: E::EvalInput<'a>,
	common_input: &E::CommonInput,
	overrides: PackageOverrides,
) -> Result<ResolutionResult, ResolutionError> {
	let mut resolver = Resolver {
		tasks: VecDeque::new(),
		constraints: Vec::new(),
		constant_input: constant_eval_input,
		package_configs: HashMap::new(),
		overrides,
	};

	// Used to keep track of which packages have been preloaded and are good to further evaluate as tasks
	let mut preloaded_packages = HashSet::with_capacity(packages.len());

	// Preload all of the user's configured packages
	let collected_packages: Vec<_> = packages
		.iter()
		.filter_map(|x| {
			let req = x.get_package();
			if resolver.overrides.suppress.contains(&req.to_string()) {
				None
			} else {
				Some(req)
			}
		})
		.collect();
	if let Err(e) = evaluator
		.preload_packages(&collected_packages, common_input)
		.await
	{
		return Err(ResolutionError::FailedToPreload(e));
	}
	preloaded_packages.extend(collected_packages);

	// Create the initial EvalPackage tasks and constraints from the installed packages
	for config in packages.iter().sorted_by_key(|x| x.get_package()) {
		let req = config.get_package();
		let props = evaluator
			.get_package_properties(&req, common_input)
			.await
			.map_err(|e| ResolutionError::FailedToGetProperties(req.clone(), e))?;

		resolver.update_require_constraint(&req, props, RequireConstraint::UserRequire)?;
		resolver.package_configs.insert(req.clone(), config.clone());
	}

	// Resolve all of the tasks
	// The strategy for preloading is to complete tasks until none of them have preloaded packages, then preload all of them and repeat
	'outer: loop {
		let mut num_skipped = 0;

		loop {
			// We have skipped all the tasks and need to finally preload them
			if num_skipped >= resolver.tasks.len() && !resolver.tasks.is_empty() {
				break;
			}

			if let Some(task) = resolver.tasks.pop_front() {
				// Skip this task if it is not preloaded
				#[allow(irrefutable_let_patterns)]
				if let Task::EvalPackage { dest, .. } = &task {
					if !preloaded_packages.contains(dest) {
						num_skipped += 1;
						resolver.tasks.push_back(task);
						continue;
					}
				}

				resolve_task(task, common_input, &mut evaluator, &mut resolver).await?;
				resolver.check_compats(&mut evaluator, common_input).await?;

				// Reset the skip count since it is no longer valid
				num_skipped = 0;
			} else {
				break 'outer;
			}
		}

		// Preload all of the packages
		let to_preload: Vec<_> = resolver
			.tasks
			.iter()
			.filter_map(|x| {
				#[allow(irrefutable_let_patterns)]
				if let Task::EvalPackage { dest, .. } = x {
					if resolver.overrides.suppress.contains(&dest.to_string()) {
						None
					} else {
						Some(dest.clone())
					}
				} else {
					None
				}
			})
			.collect();

		if let Err(e) = evaluator.preload_packages(&to_preload, common_input).await {
			return Err(ResolutionError::FailedToPreload(e));
		};

		preloaded_packages.extend(to_preload);
	}

	let mut unfulfilled_recommendations = Vec::new();

	// Final check for constraints
	for constraint in resolver.constraints.iter() {
		match &constraint.kind {
			ConstraintKind::Recommend(package, invert) => {
				if *invert {
					if resolver.is_required(package) {
						unfulfilled_recommendations.push(RecommendedPackage {
							req: package.clone(),
							invert: true,
						});
					}
				} else if !resolver.is_required(package) {
					unfulfilled_recommendations.push(RecommendedPackage {
						req: package.clone(),
						invert: false,
					});
				}
			}
			ConstraintKind::Extend(package) => {
				if !resolver.is_required(package) {
					let source = package.source.get_source();
					return Err(ResolutionError::ExtensionNotFulfilled(
						source,
						package.clone(),
					));
				}
			}
			_ => {}
		}
	}

	let out = ResolutionResult {
		packages: resolver.collect_packages(),
		unfulfilled_recommendations,
	};

	Ok(out)
}

/// Result from package resolution
pub struct ResolutionResult {
	/// The list of packages to install
	pub packages: Vec<ResolutionPackageResult>,
	/// Package recommendations that were not satisfied
	pub unfulfilled_recommendations: Vec<RecommendedPackage>,
}

/// A single package resulting from resolution
#[derive(Debug)]
pub struct ResolutionPackageResult {
	/// The request for this package. The content version probably doesn't mean anything.
	pub req: ArcPkgReq,
	/// The required content versions for this package
	pub required_content_versions: Vec<String>,
	/// The preferred content versions for this package
	pub preferred_content_versions: Vec<String>,
}

/// Recommended package that has a PkgRequest instead of a String
pub struct RecommendedPackage {
	/// Package to recommend
	pub req: ArcPkgReq,
	/// Whether to invert this recommendation to recommend against a package
	pub invert: bool,
}

/// Resolve a single task
async fn resolve_task<'a, E: PackageEvaluator<'a>>(
	task: Task,
	common_input: &E::CommonInput,
	evaluator: &mut E,
	resolver: &mut Resolver<'a, E>,
) -> Result<(), ResolutionError> {
	match task {
		Task::EvalPackage {
			dest,
			required_content_versions,
			preferred_content_versions,
		} => {
			if resolver.overrides.suppress.contains(&dest.to_string()) {
				return Ok(());
			}

			let result = resolve_eval_package(
				dest.clone(),
				required_content_versions,
				preferred_content_versions,
				common_input,
				evaluator,
				resolver,
			)
			.await
			.map_err(|e| ResolutionError::PackageContext(dest.clone(), Box::new(e)));

			let config = resolver.package_configs.get(&dest);

			// Skip errors for optional packages or dependencies of optional packages
			if let Err(e) = result {
				let Some(config) = config else {
					return Err(e);
				};

				if let Some(original_source) = dest.source.get_original_source() {
					if let Some(config) = resolver.package_configs.get(original_source) {
						if config.is_optional() {
							return Ok(());
						}
					}
				}

				if !config.is_optional() {
					return Err(e);
				}
			}
		}
	}

	Ok(())
}

/// Resolve an EvalPackage task
async fn resolve_eval_package<'a, E: PackageEvaluator<'a>>(
	package: ArcPkgReq,
	required_content_versions: Vec<String>,
	preferred_content_versions: Vec<String>,
	common_input: &E::CommonInput,
	evaluator: &mut E,
	resolver: &mut Resolver<'a, E>,
) -> Result<(), ResolutionError> {
	// Make sure that this package fits the constraints as well
	resolver.check_constraints(&package)?;

	// Get the correct EvalInput
	let properties = evaluator
		.get_package_properties(&package, common_input)
		.await
		.map_err(|e| ResolutionError::FailedToGetProperties(package.clone(), e))?;
	let config = resolver.package_configs.get(&package);
	let input = override_eval_input::<E>(
		properties,
		&resolver.constant_input,
		required_content_versions,
		preferred_content_versions,
		is_package_overridden(&package, &resolver.overrides.force),
		config,
	)?;

	let result = evaluator
		.eval_package_relations(&package, &input, common_input)
		.await
		.map_err(|e| ResolutionError::FailedToEvaluate(package.clone(), e))?;

	for conflict in result.get_conflicts().iter().sorted() {
		let req = Arc::new(PkgRequest::parse(
			conflict,
			PkgRequestSource::Refused(package.clone()),
		));
		if resolver.is_required(&req) {
			return Err(ResolutionError::IncompatiblePackage(
				req,
				vec![package.to_string().into()],
			));
		}
		resolver.constraints.push(Constraint {
			kind: ConstraintKind::Refuse(req),
		});
	}

	for dep in result.get_deps().iter().flatten().sorted() {
		let req = Arc::new(PkgRequest::parse(
			&dep.value,
			PkgRequestSource::Dependency(package.clone()),
		));
		if dep.explicit && !resolver.is_user_required(&req) {
			return Err(ResolutionError::ExplicitRequireNotFulfilled(
				req,
				package.clone(),
			));
		}
		resolver.check_constraints(&req)?;
		let props = evaluator
			.get_package_properties(&req, common_input)
			.await
			.map_err(|e| ResolutionError::FailedToGetProperties(package.clone(), e))?;
		resolver.update_require_constraint(&req, props, RequireConstraint::Require)?;
	}

	for bundled in result.get_bundled().iter().sorted() {
		let req = Arc::new(PkgRequest::parse(
			bundled,
			PkgRequestSource::Bundled(package.clone()),
		));
		resolver.check_constraints(&req)?;
		let props = evaluator
			.get_package_properties(&req, common_input)
			.await
			.map_err(|e| ResolutionError::FailedToGetProperties(package.clone(), e))?;

		resolver.update_require_constraint(&req, props, RequireConstraint::Bundle)?;
	}

	for (check_package, compat_package) in result.get_compats().iter().sorted() {
		let check_package = Arc::new(PkgRequest::parse(
			check_package,
			PkgRequestSource::Dependency(package.clone()),
		));
		let compat_package = Arc::new(PkgRequest::parse(
			compat_package,
			PkgRequestSource::Dependency(package.clone()),
		));
		if !resolver.compat_exists(check_package.clone(), compat_package.clone()) {
			resolver.constraints.push(Constraint {
				kind: ConstraintKind::Compat(check_package, compat_package),
			});
		}
	}

	for extension in result.get_extensions().iter().sorted() {
		let req = Arc::new(PkgRequest::parse(
			extension,
			PkgRequestSource::Dependency(package.clone()),
		));
		resolver.constraints.push(Constraint {
			kind: ConstraintKind::Extend(req),
		});
	}

	for recommendation in result.get_recommendations().iter().sorted() {
		let req = Arc::new(PkgRequest::parse(
			&recommendation.value,
			PkgRequestSource::Dependency(package.clone()),
		));
		resolver.constraints.push(Constraint {
			kind: ConstraintKind::Recommend(req, recommendation.invert),
		});
	}

	Ok(())
}

/// Overrides the EvalInput for a package with config
fn override_eval_input<'a, E: PackageEvaluator<'a>>(
	properties: &PackageProperties,
	constant_eval_input: &E::EvalInput<'a>,
	required_content_versions: Vec<String>,
	preferred_content_versions: Vec<String>,
	force: bool,
	config: Option<&E::ConfiguredPackage>,
) -> Result<E::EvalInput<'a>, ResolutionError> {
	let input = {
		let mut constant_eval_input = constant_eval_input.clone();
		constant_eval_input
			.set_content_versions(required_content_versions, preferred_content_versions);
		constant_eval_input.set_force(force);

		if let Some(config) = config {
			if let Err(e) =
				config.override_configured_package_input(properties, &mut constant_eval_input)
			{
				return Err(ResolutionError::Misc(e));
			}
		}
		constant_eval_input
	};

	Ok(input)
}

/// State for resolution
struct Resolver<'a, E: PackageEvaluator<'a>> {
	tasks: VecDeque<Task>,
	constraints: Vec<Constraint>,
	constant_input: E::EvalInput<'a>,
	package_configs: HashMap<ArcPkgReq, E::ConfiguredPackage>,
	overrides: PackageOverrides,
}

impl<'a, E> Resolver<'a, E>
where
	E: PackageEvaluator<'a>,
{
	fn is_required_fn(constraint: &Constraint, req: &ArcPkgReq) -> bool {
		matches!(
			&constraint.kind,
			ConstraintKind::Require(dest, ..)
			| ConstraintKind::UserRequire(dest, ..)
			| ConstraintKind::Bundle(dest, ..) if dest == req
		)
	}

	/// Whether a package has been required by an existing constraint
	pub fn is_required(&self, req: &ArcPkgReq) -> bool {
		self.constraints
			.iter()
			.any(|x| Self::is_required_fn(x, req))
	}

	/// Whether a package has been required by the user
	pub fn is_user_required(&self, req: &ArcPkgReq) -> bool {
		self.constraints.iter().any(|x| {
			matches!(&x.kind, ConstraintKind::UserRequire(dest, ..) if dest == req)
				|| matches!(&x.kind, ConstraintKind::Bundle(dest, ..) if dest == req && dest.source.is_user_bundled())
		})
	}

	/// Remove the require constraint of a package if it exists
	pub fn remove_require_constraint(&mut self, req: &ArcPkgReq) {
		let index = self
			.constraints
			.iter()
			.position(|x| matches!(&x.kind, ConstraintKind::Require(req2, ..) | ConstraintKind::UserRequire(req2, ..) | ConstraintKind::Bundle(req2, ..) if req2 == req));
		if let Some(index) = index {
			self.constraints.swap_remove(index);
		}
	}

	/// Updates a require constraint, returning an error if the package is now overconstrained
	pub fn update_require_constraint(
		&mut self,
		req: &ArcPkgReq,
		properties: &PackageProperties,
		kind: RequireConstraint,
	) -> Result<(), ResolutionError> {
		fn find_constraint<'a>(
			constraints: &'a mut [Constraint],
			req: &ArcPkgReq,
		) -> Option<(&'a mut Vec<String>, &'a mut Vec<String>)> {
			constraints.iter_mut().find_map(|x| {
				if let ConstraintKind::Require(req2, versions, preferred_versions)
				| ConstraintKind::UserRequire(req2, versions, preferred_versions)
				| ConstraintKind::Bundle(req2, versions, preferred_versions) = &mut x.kind
				{
					if req2 == req {
						Some((versions, preferred_versions))
					} else {
						None
					}
				} else {
					None
				}
			})
		}

		// Insert the constraint if it does not exist
		let just_inserted = if find_constraint(&mut self.constraints, req).is_none() {
			// Create a new constraint
			let versions = properties.content_versions.clone().unwrap_or_default();
			let kind = match kind {
				RequireConstraint::Require => {
					ConstraintKind::Require(req.clone(), versions, Vec::new())
				}
				RequireConstraint::UserRequire => {
					ConstraintKind::UserRequire(req.clone(), versions, Vec::new())
				}
				RequireConstraint::Bundle => {
					ConstraintKind::Bundle(req.clone(), versions, Vec::new())
				}
			};
			self.constraints.push(Constraint { kind });

			true
		} else {
			false
		};

		// Find existing constraint versions to update
		let (required_versions, preferred_versions) =
			find_constraint(&mut self.constraints, req).expect("Should have been inserted");

		// Update the constraint with the new versions
		// If the existing versions is already empty, that means this package just doesn't have any content versions
		if required_versions.is_empty() {
			return Ok(());
		}

		// Constrain the list of required versions or add to the list of preferred versions
		let mut new_version_preferred = false;
		let new_versions = if let VersionPattern::Prefer(preferred) = &req.content_version {
			if !preferred_versions.contains(preferred) {
				preferred_versions.push(preferred.clone());
				new_version_preferred = true;
			}
			required_versions.clone()
		} else {
			req.content_version.get_matches(required_versions)
		};

		// We have overconstrained to the point that there are no versions left
		if new_versions.is_empty() {
			return Err(ResolutionError::NoValidVersionsFound(req.clone()));
		}

		// If the number of versions is now smaller, that means a different version could be selected and we need to re-evaluate.
		// Also, if the best evaluable version has changed, we also need to re-evaluate
		if (just_inserted || new_version_preferred || new_versions.len() != required_versions.len())
			&& !is_package_overridden(req, &self.overrides.suppress)
		{
			self.tasks.push_back(Task::EvalPackage {
				dest: req.clone(),
				required_content_versions: new_versions.clone(),
				preferred_content_versions: preferred_versions.clone(),
			});
		}

		*required_versions = new_versions;

		// Upgrade requires to bundles
		let required_versions = required_versions.clone();
		let preferred_versions = preferred_versions.clone();
		if self
			.constraints
			.iter()
			.any(|x| matches!(&x.kind, ConstraintKind::Require(req2, ..) if req2 == req))
		{
			self.remove_require_constraint(req);

			self.constraints.push(Constraint {
				kind: ConstraintKind::Bundle(req.clone(), required_versions, preferred_versions),
			})
		}

		Ok(())
	}

	fn is_refused_fn(constraint: &Constraint, req: ArcPkgReq) -> bool {
		matches!(
			&constraint.kind,
			ConstraintKind::Refuse(dest) if *dest == req
		)
	}

	/// Whether a package has been refused by an existing constraint
	pub fn is_refused(&self, req: &ArcPkgReq) -> bool {
		self.constraints
			.iter()
			.any(|x| Self::is_refused_fn(x, req.clone()))
	}

	/// Get all refusers of this package
	pub fn get_refusers(&self, req: &ArcPkgReq) -> Vec<PackageID> {
		self.constraints
			.iter()
			.filter_map(|x| {
				if let ConstraintKind::Refuse(dest) = &x.kind {
					if dest == req {
						Some(
							dest.source
								.get_source()
								.map(|source| source.id.clone())
								.unwrap_or("User-refused".into()),
						)
					} else {
						None
					}
				} else {
					None
				}
			})
			.collect()
	}

	/// Whether a compat constraint exists
	pub fn compat_exists(&self, package: ArcPkgReq, compat_package: ArcPkgReq) -> bool {
		self.constraints.iter().any(|x| {
			matches!(
				&x.kind,
				ConstraintKind::Compat(src, dest) if *src == package && *dest == compat_package
			)
		})
	}

	/// Creates an error if this package is disallowed in the constraints
	pub fn check_constraints(&self, req: &ArcPkgReq) -> Result<(), ResolutionError> {
		if self.is_refused(req) {
			let refusers = self.get_refusers(req);
			return Err(ResolutionError::IncompatiblePackage(req.clone(), refusers));
		}

		Ok(())
	}

	/// Checks compat constraints to see if new constraints are needed
	pub async fn check_compats(
		&mut self,
		evaluator: &mut E,
		common_input: &E::CommonInput,
	) -> Result<(), ResolutionError> {
		let mut packages_to_require = Vec::new();
		for constraint in &self.constraints {
			if let ConstraintKind::Compat(package, compat_package) = &constraint.kind {
				if self.is_required(package) && !self.is_required(compat_package) {
					packages_to_require.push(compat_package.clone());
				}
			}
		}
		for package in packages_to_require {
			let props = evaluator
				.get_package_properties(&package, common_input)
				.await
				.map_err(|e| ResolutionError::FailedToGetProperties(package.clone(), e))?;
			self.update_require_constraint(&package, props, RequireConstraint::Require)?;
		}

		Ok(())
	}

	/// Collect all needed packages for final output
	pub fn collect_packages(self) -> Vec<ResolutionPackageResult> {
		self.constraints
			.into_iter()
			.filter_map(|x| match x.kind {
				ConstraintKind::Require(
					dest,
					required_content_versions,
					preferred_content_versions,
				)
				| ConstraintKind::UserRequire(
					dest,
					required_content_versions,
					preferred_content_versions,
				)
				| ConstraintKind::Bundle(
					dest,
					required_content_versions,
					preferred_content_versions,
				) => Some(ResolutionPackageResult {
					req: dest,
					required_content_versions,
					preferred_content_versions,
				}),
				_ => None,
			})
			.collect()
	}
}

/// A requirement for the installation of the packages
#[derive(Debug)]
struct Constraint {
	kind: ConstraintKind,
}

#[derive(Debug, PartialEq, Eq)]
enum ConstraintKind {
	Require(ArcPkgReq, Vec<String>, Vec<String>),
	UserRequire(ArcPkgReq, Vec<String>, Vec<String>),
	Refuse(ArcPkgReq),
	Recommend(ArcPkgReq, bool),
	Bundle(ArcPkgReq, Vec<String>, Vec<String>),
	Compat(ArcPkgReq, ArcPkgReq),
	Extend(ArcPkgReq),
}

/// A task that needs to be completed for resolution
enum Task {
	/// Evaluate a package and its relationships
	EvalPackage {
		dest: Arc<PkgRequest>,
		required_content_versions: Vec<String>,
		preferred_content_versions: Vec<String>,
	},
}

/// Different types of require constraints
pub enum RequireConstraint {
	/// Require by a package
	Require,
	/// Require by the user
	UserRequire,
	/// A bundle dependency
	Bundle,
}
