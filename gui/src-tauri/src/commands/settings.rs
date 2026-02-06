use crate::{commands::fmt_err, State};

use serde::{Deserialize, Serialize};

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, State>) -> Result<Settings, String> {
	let data = state.data.lock().await;

	Ok(Settings {
		base_theme: data.base_theme.clone(),
		overlay_themes: data.overlay_themes.clone(),
	})
}

#[tauri::command]
pub async fn write_settings(
	state: tauri::State<'_, State>,
	settings: Settings,
) -> Result<(), String> {
	let mut data = state.data.lock().await;

	data.base_theme = settings.base_theme;
	data.overlay_themes = settings.overlay_themes;

	fmt_err(data.write(&state.paths))?;

	Ok(())
}

/// Combination of config and launcher data to represent global settings
#[derive(Serialize, Deserialize)]
pub struct Settings {
	pub base_theme: Option<String>,
	pub overlay_themes: Vec<String>,
}
