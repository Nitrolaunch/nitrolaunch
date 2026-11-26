use std::{collections::HashMap, path::Path};

use anyhow::Context;
use nitro_core::io::{
	files::create_leading_dirs_async, json_from_file, json_to_file, json_to_file_pretty,
};
use nitro_instance::template::{
	BUILTIN_FILE_FORMAT, BUILTIN_JSON_FORMAT, ComponentApplyMode, ComponentMergeMode,
	ComponentWithSource,
};
use nitro_plugin::hook::hooks::{ApplyComponents, ApplyComponentsArg};
use nitro_shared::{id::TemplateID, output::NitroOutput, util::merge_json_objects};
use tokio::task::JoinSet;

use crate::{instance::Instance, io::paths::Paths, plugin::PluginManager, template::Template};

/// Applies and updates template components on an instance
pub async fn apply_templates(
	instance: &mut Instance,
	templates: &HashMap<TemplateID, Template>,
	plugins: &PluginManager,
	paths: &Paths,
	o: &mut impl NitroOutput,
) -> anyhow::Result<()> {
	let Some(instance_dir) = instance.dir() else {
		return Ok(());
	};
	let instance_dir = instance_dir.to_owned();
	let instance_config = instance.config().clone();

	// TODO: Resolve subtemplates / inheritance
	let mut components = HashMap::new();
	for template in instance.config().from.iter() {
		let template = templates
			.get(&TemplateID::from(template.clone()))
			.expect("Template should exist from config verification");

		for component in &template.data.components {
			components
				.entry(component.plugin.clone())
				.or_insert_with(Vec::new)
				.push(ComponentWithSource {
					component: component.clone(),
					source_dir: template.path.clone(),
				});
		}
	}

	let mut tasks = JoinSet::new();
	for (plugin, components) in components {
		let instance_dir = instance_dir.clone();
		let instance_config = instance_config.clone();
		let plugins = plugins.clone();
		let paths = paths.clone();
		let mut o = o.get_greater_copy();

		tasks.spawn(async move {
			let original_components = components.clone();

			let failed_components = if let Some(plugin) = plugin {
				let arg = ApplyComponentsArg {
					config: instance_config.clone(),
					components,
					target_dir: instance_dir.to_string_lossy().to_string(),
				};
				let result = plugins
					.call_hook_on_plugin(ApplyComponents, &plugin, &arg, &paths, &mut o)
					.await?;
				let Some(result) = result else {
					anyhow::bail!("Plugin {} did not apply components", plugin);
				};
				let result = result.result(&mut o).await?;
				result.failed_components
			} else {
				let mut failed_components = Vec::new();
				for component in components {
					let result = apply_builtin_component(
						&component,
						&instance_dir,
						ComponentMergeMode::MergeUnder,
						ComponentApplyMode::Save,
					)
					.await;
					if let Err(e) = result {
						o.display(nitro_shared::output::MessageContents::Warning(format!(
							"Failed to apply component {}: {}",
							component.component.path, e
						)));
						failed_components.push(component);
					}
				}
				failed_components
			};

			// TODO: Update only non-failed components in lockfile

			Ok::<(), anyhow::Error>(())
		});
	}

	while let Some(res) = tasks.join_next().await {
		res??;
	}

	Ok(())
}

async fn apply_builtin_component(
	component: &ComponentWithSource,
	instance_dir: &Path,
	merge_mode: ComponentMergeMode,
	apply_mode: ComponentApplyMode,
) -> anyhow::Result<()> {
	debug_assert_eq!(component.component.plugin, None);

	if component.component.format == BUILTIN_FILE_FORMAT.id {
		let src = component.source_dir.join(&component.component.path);
		let dest = instance_dir.join(&component.component.path);

		if src.is_dir() {
			copy_dir(&src, &dest, merge_mode).await?;
		} else {
			copy_file(&src, &dest, merge_mode).await?;
		}
	} else if component.component.format == BUILTIN_JSON_FORMAT.id {
		let (location_parts, json_path) = component.component.path_parts();
		let src = component.source_dir.join(location_parts);
		let dest = instance_dir.join(location_parts);
		let json_path: Vec<_> = json_path
			.split('/')
			.filter(|part| !part.is_empty())
			.collect();

		let _ = create_leading_dirs_async(&dest).await;
		let src_data: serde_json::Value = json_from_file(&src)
			.with_context(|| format!("Failed to read JSON component from {:?}", src))?;

		if !dest.exists() {
			write_json(&src_data, &dest, apply_mode)?;
		} else {
			let mut dest_data: serde_json::Value = json_from_file(&dest)
				.with_context(|| format!("Failed to read JSON component from {:?}", dest))?;
			apply_json(&src_data, &mut dest_data, &json_path, merge_mode)?;
			write_json(&dest_data, &dest, apply_mode)?;
		}
	}

	Ok(())
}

async fn copy_file(src: &Path, dest: &Path, merge_mode: ComponentMergeMode) -> anyhow::Result<()> {
	if merge_mode == ComponentMergeMode::MergeUnder && dest.exists() {
		return Ok(());
	}

	let _ = create_leading_dirs_async(dest).await;
	std::fs::copy(&src, &dest)
		.with_context(|| format!("Failed to copy file from {:?} to {:?}", src, dest))?;
	Ok(())
}

async fn copy_dir(src: &Path, dest: &Path, merge_mode: ComponentMergeMode) -> anyhow::Result<()> {
	if merge_mode == ComponentMergeMode::Replace && dest.exists() {
		std::fs::remove_dir_all(dest)
			.with_context(|| format!("Failed to remove directory {:?}", dest))?;
	}

	for entry in src.read_dir()? {
		let entry = entry?;
		let src_path = entry.path();
		let rel_path = src_path.strip_prefix(src)?;
		let dest_path = dest.join(rel_path);

		if entry.file_type()?.is_dir() {
			Box::pin(copy_dir(&src_path, &dest_path, merge_mode)).await?;
		} else {
			copy_file(&src_path, &dest_path, merge_mode).await?;
		}
	}

	Ok(())
}

fn write_json(
	data: &serde_json::Value,
	path: &Path,
	apply_mode: ComponentApplyMode,
) -> anyhow::Result<()> {
	match apply_mode {
		ComponentApplyMode::Save => {
			json_to_file(path, data).with_context(|| format!("Failed to write JSON to {:?}", path))
		}
		ComponentApplyMode::Restore => json_to_file_pretty(path, data)
			.with_context(|| format!("Failed to write JSON to {:?}", path)),
	}
}

fn apply_json(
	src_data: &serde_json::Value,
	dest_data: &mut serde_json::Value,
	json_path: &[&str],
	merge_mode: ComponentMergeMode,
) -> anyhow::Result<()> {
	let src_component = grab_json(src_data, json_path)
		.with_context(|| format!("JSON component path {:?} was not found", json_path))?;
	apply_json_at_path(src_component, dest_data, json_path, merge_mode)?;

	Ok(())
}

fn apply_json_at_path(
	src_data: &serde_json::Value,
	dest_data: &mut serde_json::Value,
	json_path: &[&str],
	merge_mode: ComponentMergeMode,
) -> anyhow::Result<()> {
	let Some((next_key, remaining_keys)) = json_path.split_first() else {
		if merge_mode == ComponentMergeMode::Replace {
			*dest_data = src_data.clone();
			return Ok(());
		}

		match merge_mode {
			ComponentMergeMode::MergeUnder => {
				if let (serde_json::Value::Object(src), serde_json::Value::Object(dest)) =
					(src_data, &*dest_data)
				{
					let mut merged = src.clone();
					merge_json_objects(&mut merged, dest.clone());
					*dest_data = serde_json::Value::Object(merged);
				} else if dest_data.is_null() {
					*dest_data = src_data.clone();
				}
			}
			ComponentMergeMode::MergeOver => {
				if let (serde_json::Value::Object(src), serde_json::Value::Object(dest)) =
					(src_data, &*dest_data)
				{
					let mut merged = dest.clone();
					merge_json_objects(&mut merged, src.clone());
					*dest_data = serde_json::Value::Object(merged);
				} else {
					*dest_data = src_data.clone();
				}
			}
			ComponentMergeMode::Skip => {}
			ComponentMergeMode::Replace => unreachable!(),
		}
		return Ok(());
	};

	match dest_data {
		serde_json::Value::Object(dest_map) => {
			let entry = dest_map
				.entry((*next_key).to_string())
				.or_insert(serde_json::Value::Null);
			apply_json_at_path(src_data, entry, remaining_keys, merge_mode)?;
		}
		serde_json::Value::Array(dest_array) => {
			let index: usize = next_key
				.parse()
				.with_context(|| format!("Invalid array index: {next_key}"))?;
			if index >= dest_array.len() {
				dest_array.resize(index + 1, serde_json::Value::Null);
			}
			apply_json_at_path(src_data, &mut dest_array[index], remaining_keys, merge_mode)?;
		}
		_ => anyhow::bail!("Cannot traverse into non-object/array JSON value"),
	}

	Ok(())
}

/// Grabs the requested JSON subdata from a JSON value
fn grab_json<'a>(data: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
	let mut current = data;
	for part in path {
		match current {
			serde_json::Value::Object(map) => {
				current = map.get(*part)?;
			}
			serde_json::Value::Array(array) => {
				let index: usize = part.parse().ok()?;
				current = array.get(index)?;
			}
			_ => return None,
		}
	}
	Some(current)
}

#[cfg(test)]
mod tests {
	use serde_json::{Value, json};

	use super::{apply_json, grab_json};
	use nitro_instance::template::ComponentMergeMode;

	#[test]
	fn grab_json_follows_nested_object_path() {
		let data = json!({"foo": {"bar": 7}});

		assert_eq!(grab_json(&data, &["foo", "bar"]), Some(&json!(7)));
	}

	#[test]
	fn grab_json_supports_array_indices() {
		let data = json!({"foo": [{"bar": 7}]});

		assert_eq!(grab_json(&data, &["foo", "0", "bar"]), Some(&json!(7)));
	}

	#[test]
	fn grab_json_returns_none_for_missing_or_invalid_paths() {
		let data = json!({"foo": [7]});

		assert_eq!(grab_json(&data, &["missing"]), None);
		assert_eq!(grab_json(&data, &["foo", "not-an-index"]), None);
		assert_eq!(grab_json(&data, &["foo", "1"]), None);
	}

	#[test]
	fn apply_json_replace_replaces_only_the_selected_destination_value() {
		let src = json!({"foo": {"bar": {"from_src": true}}});
		let mut dest = json!({"keep": 1, "foo": {"bar": {"from_dest": true}}});

		apply_json(
			&src,
			&mut dest,
			&["foo", "bar"],
			ComponentMergeMode::Replace,
		)
		.unwrap();

		assert_eq!(dest, json!({"keep": 1, "foo": {"bar": {"from_src": true}}}));
	}

	#[test]
	fn apply_json_merge_under_keeps_destination_values() {
		let src =
			json!({"foo": {"bar": {"shared": "src", "src_only": 1, "nested": {"src": true}}}});
		let mut dest =
			json!({"foo": {"bar": {"shared": "dest", "dest_only": 2, "nested": {"dest": true}}}});

		apply_json(
			&src,
			&mut dest,
			&["foo", "bar"],
			ComponentMergeMode::MergeUnder,
		)
		.unwrap();

		assert_eq!(
			dest,
			json!({"foo": {"bar": {
				"shared": "dest",
				"src_only": 1,
				"dest_only": 2,
				"nested": {"src": true, "dest": true}
			}}})
		);
	}

	#[test]
	fn apply_json_merge_over_keeps_source_values() {
		let src =
			json!({"foo": {"bar": {"shared": "src", "src_only": 1, "nested": {"src": true}}}});
		let mut dest =
			json!({"foo": {"bar": {"shared": "dest", "dest_only": 2, "nested": {"dest": true}}}});

		apply_json(
			&src,
			&mut dest,
			&["foo", "bar"],
			ComponentMergeMode::MergeOver,
		)
		.unwrap();

		assert_eq!(
			dest,
			json!({"foo": {"bar": {
				"shared": "src",
				"src_only": 1,
				"dest_only": 2,
				"nested": {"src": true, "dest": true}
			}}})
		);
	}

	#[test]
	fn apply_json_errors_when_source_path_is_missing() {
		let src = json!({"foo": {}});
		let mut dest: Value = json!({"foo": {}});

		assert!(
			apply_json(
				&src,
				&mut dest,
				&["foo", "bar"],
				ComponentMergeMode::Replace
			)
			.is_err()
		);
	}
}
