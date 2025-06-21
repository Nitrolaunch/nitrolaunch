use std::collections::HashMap;

use anyhow::Context;

/// A package with an optional version
pub type PackageAndVersion = (String, Option<String>);

/// Asynchronous function for substituting relations
pub trait RelationSubFunction: Send + 'static + Clone {
	/// Substitutes a single relationship
	fn substitute(
		&self,
		relation: &str,
		version: Option<&str>,
	) -> impl std::future::Future<Output = anyhow::Result<PackageAndVersion>> + Send;

	/// Preloads files for multiple substitutions. Does not have to be implemented
	fn preload_substitutions(
		&mut self,
		relations: &[String],
	) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
		let _ = relations;
		async { Ok(()) }
	}
}

// impl<A: AsyncFn(&str) -> anyhow::Result<String> + Send + 'static + Copy> RelationSubFunction for A {}

/// Substitutes relations with themselves
#[derive(Clone)]
pub struct RelationSubNone;

impl RelationSubFunction for RelationSubNone {
	async fn substitute(
		&self,
		relation: &str,
		version: Option<&str>,
	) -> anyhow::Result<PackageAndVersion> {
		Ok((relation.to_string(), version.map(|x| x.to_string())))
	}
}

/// Substitutes relations using a map
#[derive(Clone)]
pub struct RelationSubMap(pub HashMap<String, String>);

impl RelationSubFunction for RelationSubMap {
	async fn substitute(
		&self,
		relation: &str,
		version: Option<&str>,
	) -> anyhow::Result<PackageAndVersion> {
		let _ = version;
		Ok((
			self.0
				.get(relation)
				.cloned()
				.with_context(|| format!("Dependency {relation} was not substituted"))?,
			None,
		))
	}
}

/// Substitutes multiple relations concurrently. The resulting array will have the same length as the iterator.
pub async fn substitute_multiple(
	relations: impl Iterator<Item = &PackageAndVersion>,
	function: impl RelationSubFunction,
) -> anyhow::Result<HashMap<PackageAndVersion, PackageAndVersion>> {
	let mut tasks = tokio::task::JoinSet::new();
	for relation in relations {
		let relation = relation.clone();
		let function = function.clone();
		tasks.spawn(async move {
			Ok::<_, anyhow::Error>((
				relation.clone(),
				function
					.substitute(&relation.0, relation.1.as_deref())
					.await?,
			))
		});
	}

	let mut out = HashMap::new();

	while let Some(result) = tasks.join_next().await {
		let (key, val) = result??;
		out.insert(key, val);
	}

	Ok(out)
}
