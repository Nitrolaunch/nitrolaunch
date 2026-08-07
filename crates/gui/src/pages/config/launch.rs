use std::{path::Path, rc::Rc};

use nitrolaunch::config_crate::template::TemplateConfig;

use crate::{
	components::input::{Derivable, derived_value_owned, select::Selected, text::TextInput},
	ops::plugin_results::FetchJavaTypes,
	pages::config::ConfigState,
	prelude::*,
	util::PtrEq,
};

#[derive(PartialEq)]
pub struct LaunchConfigPage {
	pub config_state: ConfigState,
	pub parent_configs: PtrEq<[TemplateConfig]>,
}

impl Component for LaunchConfigPage {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		rect().expanded().padding(theme.gap3).child(JavaSelector {
			java: self.config_state.java,
			parent_configs: self.parent_configs.clone(),
		})
	}
}

#[derive(PartialEq)]
struct JavaSelector {
	java: State<Option<String>>,
	parent_configs: PtrEq<[TemplateConfig]>,
}

impl Component for JavaSelector {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();
		let types_query = use_query(Query::new((), FetchJavaTypes::new(back_state.clone())));

		let types = types_query.read().state().ok().cloned().unwrap_or_default();

		let all_options = types
			.iter()
			.map(|x| x.id.as_str())
			.chain(["auto", "system", "adoptium"])
			.collect::<Vec<_>>();

		let selected = match &*self.java.read() {
			Some(selected) => {
				if selected.is_empty() || !all_options.contains(&selected.as_str()) {
					JavaSelected::Custom
				} else {
					JavaSelected::Standard(selected.clone())
				}
			}
			None => JavaSelected::None,
		};
		let is_custom = selected == JavaSelected::Custom;

		let derived = derived_value_owned(self.java.read().clone(), &self.parent_configs.0, |x| {
			x.instance.launch.java.clone()
		});
		let derived_selected = match &derived {
			Some(selected) => {
				if selected.is_empty() || !all_options.contains(&selected.as_str()) {
					Some(JavaSelected::Custom)
				} else {
					Some(JavaSelected::Standard(selected.clone()))
				}
			}
			None => None,
		};

		let java2 = self.java;
		let selected2 = selected.clone();
		let selector = Dropdown::new(
			Selected::Single(selected),
			Rc::new(move |selected| {
				java2.clone().set(match selected.single() {
					JavaSelected::None => None,
					JavaSelected::Standard(kind) => Some(kind),
					JavaSelected::Custom => {
						// Clear out the custom selection
						if selected2 != JavaSelected::Custom {
							Some(String::new())
						} else {
							None
						}
					}
				});
			}),
		)
		.panel_colorway()
		.derived(derived_selected)
		.child(SelectOption::new(
			JavaSelected::None,
			"Inherit",
			Some("diagram"),
		))
		.child(
			SelectOption::new(JavaSelected::Standard("auto".into()), "Auto", Some("star"))
				.tip("Automatically select the best Java installation"),
		)
		.child(
			SelectOption::new(
				JavaSelected::Standard("system".into()),
				"System",
				Some("gear"),
			)
			.tip("Find Java on your system"),
		)
		.child(
			SelectOption::new(
				JavaSelected::Standard("adoptium".into()),
				"Adoptium",
				Some("box"),
			)
			.tip("Use Adoptium Java"),
		)
		.children(
			types
				.into_iter()
				.map(|x| SelectOption::new(JavaSelected::Standard(x.id), &x.name, Some("box"))),
		)
		.child(SelectOption::new(
			JavaSelected::Custom,
			"Custom",
			Some("folder"),
		));
		let selector = field("Java", "play", &theme, selector).tip(
			&front_state,
			"The Java installation to use for this instance",
		);

		// We can't use use_transform_optional_string because it will return None if the string is empty, but that means inherit
		let path = use_state(String::new);
		use_side_effect({
			let java = self.java;
			let mut path = path;
			move || {
				if let Some(java) = &*java.read()
					&& is_custom
				{
					path.set_if_modified(java.clone());
				}
			}
		});
		use_side_effect({
			let mut java = self.java;
			let path = path;
			move || {
				if is_custom {
					java.set_if_modified(Some(path.read().clone()));
				}
			}
		});

		let exists = Path::new(&*path.read()).exists();
		let custom_input = TextInput::new(path)
			.derived(derived)
			.maybe_input_error(!exists, "Path does not exist")
			.placeholder("Enter path...");
		let custom_input = field("Custom Java Path", "folder", &theme, custom_input).tip(
			&front_state,
			"The path to a custom Java installation. Should contain bin and lib folders.",
		);

		rect()
			.width(Size::fill())
			.child(selector)
			.maybe(is_custom, |this| this.child(custom_input))
	}
}

#[derive(PartialEq, Clone)]
enum JavaSelected {
	None,
	Standard(String),
	Custom,
}
