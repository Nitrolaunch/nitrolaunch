use crate::{commands::fmt_err, State};

use serde::{Deserialize, Serialize};

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, State>) -> Result<Settings, String> {
	let data = state.data.lock().await;

	Ok(Settings {
		selected_theme: data.theme.clone(),
	})
}

#[tauri::command]
pub async fn write_settings(
	state: tauri::State<'_, State>,
	settings: Settings,
) -> Result<(), String> {
	let mut data = state.data.lock().await;

	data.theme = settings.selected_theme;

	fmt_err(data.write(&state.paths))?;

	Ok(())
}

/// Combination of config and launcher data to represent global settings
#[derive(Serialize, Deserialize)]
pub struct Settings {
	pub selected_theme: Option<String>,
}
