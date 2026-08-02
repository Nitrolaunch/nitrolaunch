use anyhow::Context;
use nitrolaunch::config::{
	modifications::apply_modifications_and_write, preferences::ConfigPreferences,
};

use crate::{ops::query_spawn, simple_mutation, simple_query, util::NotEq};

simple_query!(
	name = FetchPreferences,
	ok = ConfigPreferences,
	err = anyhow::Error,
	keys = (),
	fn run(&self, _: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();

		query_spawn(back_state.0.clone(), async move {
			let config = back_state.config().await.context("Failed to get config")?;

			Ok(config.prefs.clone())
		})
	}
);

simple_mutation!(
	name = SavePreferences,
	ok = (),
	err = anyhow::Error,
	keys = NotEq<ConfigPreferences>,
	fn run(&self, prefs: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let prefs = prefs.clone();

		query_spawn(back_state.0.clone(), async move {
			let mut config = back_state.raw_config().await.context("Failed to get config")?;
			config.preferences.language = prefs.0.language;
			apply_modifications_and_write(
				&mut config,
				Vec::new(),
				&back_state.paths,
				&back_state.plugins,
				&mut back_state.output()
			)
				.await
				.context("Failed to save config")?;

			Ok(())
		})
	}
);
