use std::collections::HashMap;

use nitro_config::template::TemplateConfig;
use nitro_shared::{
	id::TemplateID,
	output::{MessageContents, MessageLevel, NitroOutput},
};

/// Consolidates template configs into the full templates
pub fn consolidate_template_configs(
	templates: HashMap<TemplateID, TemplateConfig>,
	base_template: Option<&TemplateConfig>,
	o: &mut impl NitroOutput,
) -> HashMap<TemplateID, TemplateConfig> {
	let mut out: HashMap<_, TemplateConfig> = HashMap::with_capacity(templates.len());

	let max_iterations = 10000;

	// We do this by repeatedly finding a template with an already resolved ancenstor
	let mut i = 0;
	while out.len() != templates.len() {
		for (id, template) in &templates {
			// Don't redo templates that are already done
			if out.contains_key(id) {
				continue;
			}

			if template.instance.from.is_empty() {
				// Templates with no ancestor can just be added directly to the output, after deriving from the base template
				let mut template = template.clone();
				if let Some(base_template) = base_template {
					let overlay = template;
					template = base_template.clone();
					template.merge(overlay);
				}
				out.insert(id.clone(), template);
			} else {
				for parent in template.instance.from.iter() {
					// If the parent is already in the map (already consolidated) then we can derive from it and add to the map
					if let Some(parent) = out.get(&TemplateID::from(parent.clone())) {
						let mut new = parent.clone();
						new.merge(template.clone());
						out.insert(id.clone(), new);
					} else {
						let message = if templates.contains_key(parent.as_str()) {
							format!("Cyclic template structure found")
						} else {
							format!("Parent template '{parent}' does not exist")
						};
						o.display(MessageContents::Error(message), MessageLevel::Important);

						continue;
					}
				}
			}
		}

		i += 1;
		if i > max_iterations {
			panic!(
				"Max iterations exceeded while resolving templates. This is a bug in Nitrolaunch."
			);
		}
	}

	out
}
