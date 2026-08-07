use std::rc::Rc;

use nitrolaunch::shared::{
	loaders::Loader,
	pkg::{PackageCategory, PackageKind},
};

use crate::{
	components::input::select::Selected,
	ops::{plugin_results::FetchSupportedLoaders, versions::FetchMinecraftVersions},
	prelude::*,
	util::{
		assets::{get_loader_icon, get_package_kind_icon},
		pkg::{PACKAGE_CATEGORIES, package_category_display_name, package_category_icon},
	},
};

#[derive(PartialEq)]
pub struct PackageFilters {
	pub loaders: State<Vec<Loader>>,
	pub mc_versions: State<Vec<String>>,
	pub categories: State<Vec<PackageCategory>>,
	pub on_reset: EventHandler<()>,
}

impl Component for PackageFilters {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_open = use_state(|| false);

		let on_reset = self.on_reset.clone();
		let reset_button = icon_text_button("refresh", "Reset Filters", &theme)
			.width(Size::fill())
			.on_press(move |_| {
				on_reset.call(());
			});

		let loaders_filter = PackageLoadersFilter {
			loaders: self.loaders,
		};
		let loaders_filter = field("Loaders", "box", &theme, loaders_filter);

		let mc_versions_filter = PackageVersionsFilter {
			mc_versions: self.mc_versions,
		};
		let mc_versions_filter = field(
			"Minecraft Versions",
			"minecraft",
			&theme,
			mc_versions_filter,
		);

		let categories_filter = PackageCategoryFilter {
			categories: self.categories,
			repo: None,
		};
		let categories_filter = field("Categories", "tag", &theme, categories_filter);
		let dropdown = rect()
			.width(Size::px(240.0))
			.position(
				Position::new_absolute()
					.top(theme.input_height + theme.gap2)
					.right(0.0),
			)
			.layer(Layer::OverlayLevel(1))
			.panel_colorway(&theme, false, false)
			.padding(theme.gap3)
			.corner_radius(theme.round)
			.child(loaders_filter)
			.child(mc_versions_filter)
			.child(categories_filter)
			.child(reset_button);

		let mut is_open2 = is_open;
		let button = icon_button("text_align_center", &theme).on_press(move |_| {
			is_open2.toggle();
		});

		rect()
			.center()
			.child(button)
			.maybe(*is_open.read(), |this| this.child(dropdown))
	}
}

#[derive(PartialEq)]
pub struct PackageTypeFilter {
	pub ty: State<PackageKind>,
	pub repo: Option<String>,
}

impl Component for PackageTypeFilter {
	fn render(&self) -> impl IntoElement {
		let back_state = use_consume::<BackState>();

		let available_types = self
			.repo
			.as_ref()
			.and_then(|repo| back_state.repos().get(repo))
			.map(|r| r.package_types.clone())
			.unwrap_or_default();

		Dropdown::from_state(self.ty)
			.panel_colorway()
			.header_width(Size::px(180.0))
			.children(
				[
					PackageKind::Mod,
					PackageKind::ResourcePack,
					PackageKind::Datapack,
					PackageKind::Plugin,
					PackageKind::Shader,
					PackageKind::Modpack,
				]
				.into_iter()
				.filter(|x| available_types.is_empty() || available_types.contains(x))
				.map(|x| {
					SelectOption::new(
						x,
						&format!("{}s", x.to_string_pretty()),
						Some(get_package_kind_icon(x)),
					)
				}),
			)
	}
}

#[derive(PartialEq)]
pub struct PackageCategoryFilter {
	pub categories: State<Vec<PackageCategory>>,
	pub repo: Option<String>,
}

impl Component for PackageCategoryFilter {
	fn render(&self) -> impl IntoElement {
		let back_state = use_consume::<BackState>();

		let categories = self.categories;
		let available_categories = self
			.repo
			.as_ref()
			.and_then(|repo| back_state.repos().get(repo))
			.map(|r| r.package_categories.clone())
			.unwrap_or_default();

		Dropdown::new(
			Selected::Multi(categories.read().clone()),
			Rc::new(move |selected| {
				categories.clone().set(selected.multi());
			}),
		)
		.panel_colorway()
		.children(
			PACKAGE_CATEGORIES
				.iter()
				.filter(|x| available_categories.is_empty() || available_categories.contains(x))
				.map(|x| {
					SelectOption::new(
						*x,
						package_category_display_name(*x),
						Some(package_category_icon(*x)),
					)
				}),
		)
	}
}

#[derive(PartialEq)]
pub struct PackageVersionsFilter {
	pub mc_versions: State<Vec<String>>,
}

impl Component for PackageVersionsFilter {
	fn render(&self) -> impl IntoElement {
		let back_state = use_consume::<BackState>();
		let versions_query = use_query(FetchMinecraftVersions::new(back_state, false));

		let default = Vec::new();
		let available_versions = versions_query.read();
		let available_versions = available_versions.state();
		let available_versions = available_versions.ok().unwrap_or(&default);

		let mc_versions = self.mc_versions;
		Dropdown::new(
			Selected::Multi(mc_versions.read().clone()),
			Rc::new(move |selected| {
				mc_versions.clone().set(selected.multi());
			}),
		)
		.panel_colorway()
		.children(
			available_versions
				.iter()
				.rev()
				.map(|x| SelectOption::simple(x.clone())),
		)
	}
}

#[derive(PartialEq)]
pub struct PackageLoadersFilter {
	pub loaders: State<Vec<Loader>>,
}

impl Component for PackageLoadersFilter {
	fn render(&self) -> impl IntoElement {
		let back_state = use_consume::<BackState>();
		let supported_loaders = use_query(FetchSupportedLoaders::new(back_state.clone()));
		let supported_loaders = supported_loaders
			.read()
			.state()
			.ok()
			.cloned()
			.unwrap_or_default();

		let loaders = self.loaders;
		Dropdown::new(
			Selected::Multi(loaders.read().clone()),
			Rc::new(move |selected| {
				loaders.clone().set(selected.multi());
			}),
		)
		.panel_colorway()
		.children(supported_loaders.iter().map(|x| {
			SelectOption::new_custom_icon(
				x.clone(),
				&x.to_string(),
				get_loader_icon(x).into_element(),
			)
		}))
	}
}
