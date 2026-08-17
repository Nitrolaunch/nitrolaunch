use std::{
	collections::{HashMap, HashSet},
	hash::{DefaultHasher, Hash, Hasher},
	rc::Rc,
	sync::Arc,
};

use itertools::Itertools;
use nitrolaunch::{
	config_crate::{
		ConfigKind,
		template::{TemplateConfig, TemplatePackageConfiguration},
	},
	instance_crate::{addon::Addon, lock::InstanceLockfile},
	pkg_crate::{PkgRequest, PkgRequestSource},
	shared::{
		Side,
		pkg::{ArcPkgReq, PackageKind},
		util::{from_string_json, to_string_json},
		versions::VersionPattern,
	},
};

use crate::{
	components::{
		input::{
			final_value_owned,
			select::Selected,
			text::{TextInput, search_bar},
		},
		pkg::{error::ResolutionErrorView, versions::InstalledPackageVersion},
	},
	ops::{
		ConditionalQuery,
		instance::{UpdateInstance, UpdateInstanceKeys, UpdateInstanceMode},
		packages::{
			FetchInstanceAddons, FetchInstanceLockfile, FetchPackages, PkgInfo, PreloadPackages,
		},
	},
	pages::{config::ConfigState, package::browse::BrowseFilters},
	prelude::*,
	routing::Page,
	state::use_launcher_data,
	util::{PtrEq, assets::get_package_kind_icon},
};

#[derive(PartialEq)]
pub struct ContentConfig {
	pub config_state: ConfigState,
	pub parent_configs: PtrEq<[TemplateConfig]>,
	pub on_edit: Option<EventHandler<()>>,
}

impl Component for ContentConfig {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();
		let lockfile = use_query(ConditionalQuery::new(
			FetchInstanceLockfile::new(back_state.clone()),
			self.config_state.ty == ConfigKind::Instance,
			|| self.config_state.id.read().cloned(),
		));
		let addons = use_query(ConditionalQuery::new(
			FetchInstanceAddons::new(back_state.clone()),
			self.config_state.ty == ConfigKind::Instance,
			|| self.config_state.id.read().cloned(),
		));
		let update = use_mutation(Mutation::new(UpdateInstance::new(back_state.clone())));
		let data = use_launcher_data();

		let filter = use_state(|| Filter::Configured);
		let pkg_ty = use_state::<Option<PackageKind>>(|| None);
		let search = use_state(String::new);
		// let side = use_state::<Option<Side>>(|| None);

		let open_states = use_state::<HashSet<String>>(HashSet::new);

		let modpack = self.config_state.modpack;
		let packages = self.config_state.packages;
		let parent_configs = self.parent_configs.clone();
		let results = use_memo(move || {
			build_items(
				lockfile.read().state().ok(),
				modpack.read().as_ref(),
				&packages.read(),
				&parent_configs.0,
				addons.read().state().ok().map(|x| x.as_slice()),
			)
		});

		let preload = use_mutation(PreloadPackages::new(back_state.clone()));
		let ty = self.config_state.ty;
		use_side_effect(move || {
			let results = results.read();
			if lockfile.read().state().is_ok() || ty != ConfigKind::Instance {
				preload.mutate(results.1.clone());
			}
		});

		let packages = use_query(ConditionalQuery::new(
			FetchPackages::new(back_state.clone()),
			preload.read().state().is_ok(),
			move || results.read().1.clone(),
		));
		let default_packages = Arc::new(HashMap::new());

		let packages = use_memo(move || {
			let packages = packages.read();
			let packages = packages.state();
			PtrEq(packages.ok().unwrap_or(&default_packages).clone())
		});

		let packages2 = packages;
		let processed_items = use_memo(move || {
			filter_sort_items(
				results.read().0.clone(),
				&packages2.read().0,
				&filter.read(),
				&search.read(),
				pkg_ty.read().as_ref(),
			)
		});

		let items = if processed_items.read().is_empty() {
			rect()
				.expanded()
				.child(placeholder(
					"Nothing to see here. Try changing your filters or adding some packages.",
					&theme,
				))
				.into_element()
		} else {
			let config_state = self.config_state.clone();
			let processed_items2 = processed_items;
			let open_states2 = open_states;
			let open_states3 = open_states;
			let theme2 = theme.clone();
			let on_edit = self.on_edit.clone();
			VirtualScrollView::new_with_data(
				(
					packages.read().cloned(),
					processed_items.read().cloned(),
					open_states.read().cloned(),
				),
				move |item, (packages, processed_items, open_states)| {
					let item = processed_items.get(item.index).unwrap();
					let open_states2 = open_states2;
					let id = item.id.to_string();
					let open_toggle = EventHandler::new(move |_: ()| {
						let contains = open_states2.read().contains(&id);
						if contains {
							open_states2.clone().write().remove(&id);
						} else {
							open_states2.clone().write().insert(id.clone());
						}
					});

					ContentItemElem {
						item: item.clone(),
						packages: packages.0.clone(),
						config_state: config_state.clone(),
						is_open: open_states.contains(&item.id.to_string()),
						on_edit: on_edit.clone(),
						open_toggle,
					}
					.into_element()
				},
			)
			.expanded()
			.item_size(move |i| {
				let height = processed_items2
					.read()
					.get(i)
					.map(|x| {
						ContentItemElem::height(x, open_states3.read().contains(&x.id.to_string()))
					})
					.unwrap_or(ContentItemElem::base_height(false));

				height + theme2.gap
			})
			.length(processed_items.read().len())
			.into_element()
		};

		let filters = Dropdown::from_state(filter)
			.header_width(Size::flex(1.0))
			.child(SelectOption::new(
				Filter::Configured,
				"Configured",
				Some("gear"),
			))
			.child(SelectOption::new(
				Filter::Dependencies,
				"Dependencies",
				Some("scale"),
			))
			.child(SelectOption::new(Filter::All, "All", Some("asterisk")));

		let pkg_ty2 = pkg_ty;
		let ty_selector = Dropdown::new(
			Selected::Single(*pkg_ty.read()),
			Rc::new(move |selected| {
				pkg_ty2.clone().set(selected.single());
			}),
		)
		.header_width(Size::flex(1.0))
		.child(SelectOption::new(None, "Any Type", Some("asterisk")))
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
			.map(|x| SelectOption::new(x, x.to_string_pretty(), Some(get_package_kind_icon(x)))),
		);

		let search_input = search_bar(
			TextInput::new(search).placeholder("Search configured and installed content..."),
			&theme,
		);

		// let on_select_side =
		// 	Rc::new(move |new_side: Selected<Option<Side>>| side.clone().set(new_side.single()));
		// let sides = Dropdown::new(Selected::Single(side.read().clone()), on_select_side)
		// 	.header_width(Size::flex(1.0))
		// 	.options_width(180.0)
		// 	.child(SelectOption::new(None, "Any Side", Some("asterisk")))
		// 	.child(SelectOption::new(
		// 		Some(Side::Client),
		// 		"Client",
		// 		Some("controller"),
		// 	))
		// 	.child(SelectOption::new(
		// 		Some(Side::Server),
		// 		"Server",
		// 		Some("server"),
		// 	));

		let front_state2 = front_state.clone();
		let back_state2 = back_state.clone();
		let parent_configs = self.parent_configs.clone();
		let id = self.config_state.id;
		let version = self.config_state.version;
		let loader = match self.config_state.side.read().as_ref() {
			Some(Side::Client) | None => self.config_state.client_loader,
			Some(Side::Server) => self.config_state.server_loader,
		};
		let ty = self.config_state.ty;
		let is_dirty = self.config_state.is_dirty.clone();
		let browse_button = icon_text_button("search", "Browse for Packages", &theme)
			.border_fill(theme.primary)
			.color(theme.primary)
			.background(theme.primary_bg)
			.hover_background(theme.primary_bg)
			.on_press(move |_| {
				let front_state2 = front_state2.clone();
				let back_state2 = back_state2.clone();
				let parent_configs = parent_configs.clone();
				let id = id;
				let version = version;
				let loader = loader;
				spawn(async move {
					if *is_dirty.read() {
						front_state2
							.write()
							.toast(Toast::warning("You have unsaved changes", None));
						return;
					}

					let id = id.read().clone();
					let version =
						final_value_owned(version.read().clone(), &parent_configs.0, |x| {
							x.instance.version.as_ref().map(to_string_json)
						});
					let Some(version) = version else {
						return;
					};
					let canonical_version = tokio::spawn(async move {
						let version = from_string_json(&version).ok()?;
						back_state2
							.canonicalize_version(Some(&id), ty, &version)
							.await
					})
					.await;
					let Ok(Some(canonical_version)) = canonical_version else {
						front_state2
							.write()
							.toast(Toast::error("Failed to get canonical version", None));
						return;
					};

					front_state2
						.write()
						.navigate(Page::Packages(Some(BrowseFilters {
							mc_versions: vec![canonical_version.to_string()],
							loader: loader.read().clone().unwrap_or_default(),
						})));
				});
			});

		let id = self.config_state.id;
		let is_dirty = self.config_state.is_dirty.clone();
		let front_state2 = front_state.clone();
		let update_button = icon_text_button("cycle", "Update Packages", &theme)
			.active(&theme)
			.on_press(move |_| {
				if *is_dirty.read() {
					front_state2
						.write()
						.toast(Toast::warning("You have unsaved changes", None));
					return;
				}

				update.mutate(UpdateInstanceKeys {
					id: id.read().clone(),
					mode: UpdateInstanceMode::Packages,
					force: false,
				});
			});

		let controls = rect()
			.width(Size::fill())
			.cont()
			.child(segment(browse_button, 1.0))
			.child(segment(rect(), 1.0))
			.child(segment(update_button, 1.0).cross_align(Alignment::End));
		let filters = rect()
			.width(Size::fill())
			.cont()
			.child(segment(filters, 1.0))
			.child(segment(search_input, 3.0))
			.child(segment(ty_selector, 1.0));
		let header = rect()
			.width(Size::fill())
			.spacing(theme.gap)
			.child(controls)
			.child(filters);

		let resolution_error = data
			.data
			.read()
			.last_resolution_errors
			.get(&*self.config_state.id.read())
			.map(|x| ResolutionErrorView {
				error: PtrEq(Arc::new(x.clone())),
			});

		rect()
			.expanded()
			.spacing(theme.gap)
			.padding(theme.gap2)
			.child(header)
			.maybe_child(resolution_error)
			.child(items)
	}
}

#[derive(PartialEq, Clone)]
enum Filter {
	Configured,
	Dependencies,
	All,
}

struct ContentItemElem {
	item: ContentItem,
	packages: Arc<HashMap<ArcPkgReq, anyhow::Result<PkgInfo>>>,
	config_state: ConfigState,
	is_open: bool,
	open_toggle: EventHandler<()>,
	on_edit: Option<EventHandler<()>>,
}

impl ContentItemElem {
	fn base_height(is_modpack: bool) -> f32 {
		if is_modpack { 88.0 } else { 64.0 }
	}

	fn height(item: &ContentItem, is_open: bool) -> f32 {
		let mut height = Self::base_height(item.is_modpack());
		if is_open {
			let len = if item.is_modpack() {
				item.locked_packages.0.len()
			} else {
				item.locked_addons.0.len()
			};
			height += len as f32 * SubItem::height();
		}

		height
	}
}

impl PartialEq for ContentItemElem {
	fn eq(&self, other: &Self) -> bool {
		self.item == other.item
			&& self.is_open == other.is_open
			&& Arc::ptr_eq(&self.packages, &other.packages)
	}
}

impl Component for ContentItemElem {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();
		let update = use_mutation(Mutation::new(UpdateInstance::new(back_state.clone())));

		let is_hovered = use_state(|| false);

		let info = self
			.item
			.req()
			.and_then(|req| self.packages.get(req).and_then(|x| x.as_ref().ok()));

		let default_icon = icon("box", 32.0).into_element();

		let (ico, name, id) = match &self.item.ty {
			ContentItemType::Modpack { req } | ContentItemType::Package { req } => {
				if let Some(info) = info {
					let ico = info
						.meta
						.icon
						.as_deref()
						.map(|x| {
							img(x)
								.width(Size::px(40.0))
								.height(Size::px(40.0))
								.corner_radius(theme.round)
								.into_element()
						})
						.unwrap_or(default_icon);

					(
						ico,
						self.item.get_name(&self.packages).to_string(),
						Some(self.item.id.to_string()),
					)
				} else if let Some(Err(err)) = self.packages.get(req) {
					(
						icon("error", 24.0).color(theme.error).into_element(),
						err.root_cause().to_string(),
						Some(self.item.id.to_string()),
					)
				} else {
					(
						CircularLoader::new().into_element(),
						"Loading".into(),
						Some(self.item.id.to_string()),
					)
				}
			}
			ContentItemType::Addon => {
				let ico = self
					.item
					.addon_ty
					.map(get_package_kind_icon)
					.map(|x| icon(x, 24.0).into_element())
					.unwrap_or(default_icon);

				(ico, self.item.id.to_string(), None)
			}
		};

		let header_size = 16.0 + theme.gap * 2.0;
		let config_state2 = self.config_state.clone();
		let item2 = self.item.clone();
		let on_edit = self.on_edit.clone();
		let more_dropdown = Dropdown::new(
			Selected::Single(ItemMoreDropdown::More),
			Rc::new(move |selected| match selected.single() {
				ItemMoreDropdown::More => {}
				ItemMoreDropdown::Remove => match &item2.ty {
					ContentItemType::Modpack { .. } | ContentItemType::Addon => {}
					ContentItemType::Package { req } => {
						config_state2.packages.clone().write().remove_package(req);
						if let Some(on_edit) = &on_edit {
							on_edit.call(());
						}
					}
				},
			}),
		)
		.custom_header(SelectOption::new(
			ItemMoreDropdown::More,
			"",
			Some("ellipsis"),
		))
		.header_width(Size::px(header_size))
		.hide_arrow()
		.options_width(180.0)
		.options_position(Position::new_absolute().right(header_size + theme.gap))
		.child(SelectOption::new(
			ItemMoreDropdown::Remove,
			"Remove",
			Some("trash"),
		));

		let mut badges = Vec::new();

		if let ContentItemType::Package { req } = &self.item.ty {
			badges.push(
				InstalledPackageVersion {
					configured: req.content_version.optional().map(|x| x.to_string()),
					installed: self.item.locked_version.clone(),
				}
				.into_element(),
			);
		}

		if self.item.is_configured && !(self.item.is_locked && self.item.files_exist) {
			badges.push(
				badge("warning", theme.warning, &theme)
					.tip(
						&front_state,
						"Item is not yet installed. You may need to update your instance.",
					)
					.into_element(),
			);
		}
		if self.item.is_derived {
			badges.push(
				badge("diagram", theme.template, &theme)
					.background(theme.template_bg)
					.tip(&front_state, "Inherited from a template")
					.into_element(),
			);
		}
		if !self.item.is_configured && self.item.is_locked {
			badges.push(
				badge("scale", theme.fg3, &theme)
					.tip(&front_state, "Dependency of another package")
					.into_element(),
			);
		}
		if let Some(kind) = self.item.get_addon_ty(&self.packages) {
			badges.push(
				badge(get_package_kind_icon(kind), theme.fg3, &theme)
					.tip(&front_state, kind.to_string_pretty())
					.into_element(),
			);
		}

		if self.item.is_package() && self.item.is_configured && !self.item.is_derived {
			badges.push(more_dropdown.into_element());
		}

		if self.item.is_modpack()
			&& self.config_state.ty == ConfigKind::Instance
			&& !self.config_state.is_new
		{
			let id = self.config_state.id.read().clone();
			let update_button = icon_button("cycle", &theme)
				.active(&theme)
				.on_press(move |_| {
					update.mutate(UpdateInstanceKeys {
						id: id.clone(),
						mode: UpdateInstanceMode::Modpack,
						force: false,
					});
				});
			let update_button = rect()
				.tip(&front_state, "Update the modpack and all packages")
				.child(update_button);
			badges.push(update_button.into_element());
		}

		let header_height = Self::base_height(self.item.is_modpack());
		let open_toggle = self.open_toggle.clone();
		let header = rect()
			.width(Size::fill())
			.height(Size::px(header_height))
			.cont()
			.corner_radius(theme.round2)
			.panel_colorway(&theme, *is_hovered.read(), false)
			.hover(is_hovered)
			.on_press(move |_| open_toggle.call(()))
			.child(
				rect()
					.width(Size::px(header_height))
					.height(Size::fill())
					.center()
					.child(ico),
			)
			.child(
				rect()
					.width(Size::flex(4.0))
					.height(Size::fill())
					.cont()
					.vertical()
					.main_align(Alignment::Center)
					.cross_align(Alignment::Start)
					.child(name)
					.maybe(id.is_some(), |this| {
						this.child(label().text(id.unwrap()).color(theme.fg3))
					}),
			)
			.child(
				rect()
					.width(Size::flex(4.0))
					.height(Size::fill())
					.cont()
					.main_align(Alignment::End)
					.cross_align(Alignment::Center)
					.padding(Gaps::new(0.0, 20.0, 0.0, 0.0))
					.children(badges),
			);

		let subitems = rect()
			.width(Size::fill())
			.maybe(self.item.is_package(), |this| {
				this.children(
					self.item
						.locked_addons
						.0
						.iter()
						.map(|x| SubItem::from_addon(x).render(&theme)),
				)
			})
			.maybe(self.item.is_modpack(), |this| {
				this.children(
					self.item
						.locked_packages
						.0
						.iter()
						.map(|x| SubItem::from_pkg(x, &self.packages, &theme).render(&theme)),
				)
			});

		rect()
			.width(Size::fill())
			.margin(Gaps::new(0.0, 0.0, theme.gap, 0.0))
			.panel_colorway(&theme, false, false)
			.corner_radius(theme.round2)
			.child(header)
			.maybe(self.is_open, |this| this.child(subitems))
	}

	fn render_key(&self) -> DiffKey {
		let mut key = DefaultHasher::new();
		self.item.id.hash(&mut key);
		DiffKey::U64(key.finish())
	}
}

#[derive(PartialEq, Clone)]
enum ItemMoreDropdown {
	More,
	Remove,
}

#[derive(PartialEq, Clone, PartialOrd, Eq, Ord, Debug)]
struct ContentItem {
	ty: ContentItemType,
	id: Arc<str>,
	is_configured: bool,
	is_locked: bool,
	is_derived: bool,
	files_exist: bool,
	locked_version: Option<String>,
	locked_addons: PtrEq<[Addon]>,
	locked_packages: PtrEq<[ArcPkgReq]>,
	addon_ty: Option<PackageKind>,
}

impl ContentItem {
	fn is_package(&self) -> bool {
		matches!(&self.ty, ContentItemType::Package { .. })
	}

	fn is_modpack(&self) -> bool {
		matches!(&self.ty, ContentItemType::Modpack { .. })
	}

	fn req(&self) -> Option<&ArcPkgReq> {
		match &self.ty {
			ContentItemType::Package { req } | ContentItemType::Modpack { req } => Some(req),
			ContentItemType::Addon => None,
		}
	}

	fn get_name<'a>(&'a self, info: &'a HashMap<ArcPkgReq, anyhow::Result<PkgInfo>>) -> &'a str {
		if let Some(req) = self.req() {
			if let Some(Ok(info)) = info.get(req) {
				info.meta.name.as_deref().unwrap_or(&self.id)
			} else {
				&self.id
			}
		} else {
			&self.id
		}
	}

	fn get_addon_ty(
		&self,
		info: &HashMap<ArcPkgReq, anyhow::Result<PkgInfo>>,
	) -> Option<PackageKind> {
		if let Some(req) = self.req() {
			if let Some(Ok(info)) = info.get(req) {
				info.props.kinds.first().copied()
			} else {
				self.addon_ty
			}
		} else {
			self.addon_ty
		}
	}
}

#[derive(PartialEq, Clone, PartialOrd, Eq, Ord, Debug)]
enum ContentItemType {
	Modpack { req: ArcPkgReq },
	Package { req: ArcPkgReq },
	Addon,
}

struct SubItem {
	image: Element,
	name: String,
}

impl SubItem {
	fn height() -> f32 {
		40.0
	}

	fn from_addon(addon: &Addon) -> Self {
		let ico = get_package_kind_icon(PackageKind::from_addon_kind(addon.kind));
		let ico = icon(ico, 16.0);
		Self {
			image: ico.into_element(),
			name: addon.file_name.clone(),
		}
	}

	fn from_pkg(
		req: &ArcPkgReq,
		info: &HashMap<ArcPkgReq, anyhow::Result<PkgInfo>>,
		theme: &Theme,
	) -> Self {
		let ico = info
			.get(req)
			.and_then(|x| x.as_ref().ok())
			.and_then(|x| x.meta.icon.as_deref())
			.map(|x| {
				img(x)
					.width(Size::px(24.0))
					.height(Size::px(24.0))
					.corner_radius(theme.round)
					.into_element()
			})
			.unwrap_or_else(|| icon("box", 16.0).into_element());

		let name = info
			.get(req)
			.and_then(|x| x.as_ref().ok())
			.and_then(|x| x.meta.name.clone())
			.unwrap_or_else(|| req.to_string_no_version());

		Self { image: ico, name }
	}

	fn render(self, theme: &Theme) -> impl IntoElement {
		let height = Self::height();

		let ico = rect()
			.width(Size::px(height))
			.height(Size::px(height))
			.center()
			.child(self.image);

		let name = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.main_align(Alignment::Center)
			.child(self.name);

		rect()
			.width(Size::fill())
			.height(Size::px(height))
			.cont()
			.margin(Gaps::new(0.0, 0.0, 0.0, theme.gap2))
			.child(ico)
			.child(name)
	}
}

fn build_items(
	lockfile: Option<&InstanceLockfile>,
	modpack: Option<&String>,
	configured_packages: &TemplatePackageConfiguration,
	parent_configs: &[TemplateConfig],
	addons: Option<&[Addon]>,
) -> (Vec<ContentItem>, Vec<ArcPkgReq>) {
	let mut items = Vec::new();
	let mut packages = HashSet::new();

	if let Some(modpack) = modpack {
		let lock_modpack = lockfile.and_then(|x| x.get_modpack());
		let req = PkgRequest::parse(modpack, PkgRequestSource::UserRequire).arc();
		let addons = if let Some(lockfile) = lockfile {
			PtrEq(
				lockfile
					.get_addons()
					.filter(|x| x.from_modpack)
					.map(|x| x.to_addon())
					.collect(),
			)
		} else {
			PtrEq(Arc::default())
		};

		let locked_packages = lock_modpack
			.map(|x| {
				PtrEq(
					x.packages
						.iter()
						.map(|x| PkgRequest::parse(x, PkgRequestSource::UserRequire).arc())
						.collect(),
				)
			})
			.unwrap_or_else(|| PtrEq(Arc::default()));

		items.push(ContentItem {
			ty: ContentItemType::Modpack { req: req.clone() },
			id: Arc::from(modpack.clone()),
			is_configured: true,
			is_locked: lock_modpack.is_some(),
			is_derived: false,
			files_exist: true,
			locked_version: None,
			locked_addons: addons,
			locked_packages,
			addon_ty: None,
		});

		packages.insert(req);
		if let Some(lock_modpack) = lock_modpack {
			packages.extend(
				lock_modpack
					.packages
					.iter()
					.map(|x| PkgRequest::parse(x, PkgRequestSource::UserRequire).arc()),
			);
		}
	}

	if let Some(lockfile) = &lockfile {
		for (pkg, data) in lockfile.get_packages() {
			let req = PkgRequest::parse(pkg, PkgRequestSource::UserRequire)
				.with_content_version(
					data.content_version
						.as_deref()
						.map(VersionPattern::from)
						.unwrap_or_default(),
				)
				.arc();

			let mut files_exist = true;
			let addons = lockfile
				.get_addons()
				.filter(|x| x.is_from_package(pkg))
				.map(|x| x.to_addon())
				.inspect(|x| {
					if !x.exists() {
						files_exist = false;
					}
				});

			items.push(ContentItem {
				ty: ContentItemType::Package { req: req.clone() },
				id: Arc::from(pkg.clone()),
				is_locked: true,
				is_configured: false,
				is_derived: false,
				locked_version: data.content_version.clone(),
				locked_addons: PtrEq(addons.collect()),
				locked_packages: PtrEq(Arc::default()),
				files_exist,
				addon_ty: None,
			});
			packages.insert(req);
		}
	}

	let global_packages = configured_packages.iter_global().map(|x| (x, None));
	let client_packages = configured_packages
		.iter_side(Side::Client)
		.map(|x| (x, Some(Side::Client)));
	let server_packages = configured_packages
		.iter_side(Side::Server)
		.map(|x| (x, Some(Side::Server)));
	for (package, _side) in global_packages
		.chain(client_packages)
		.chain(server_packages)
	{
		let req = PkgRequest::parse(package.get_pkg_id(), PkgRequestSource::UserRequire).arc();
		let item = if let Some(pos) = items
			.iter()
			.position(|x| matches!(&x.ty, ContentItemType::Package {req: req2} if *req2 == req))
		{
			&mut items[pos]
		} else {
			items.push(ContentItem {
				ty: ContentItemType::Package { req: req.clone() },
				id: package.get_pkg_id(),
				is_configured: true,
				is_locked: false,
				is_derived: false,
				files_exist: false,
				locked_version: None,
				locked_addons: PtrEq(Arc::default()),
				locked_packages: PtrEq(Arc::default()),
				addon_ty: None,
			});
			items.last_mut().unwrap()
		};
		item.is_configured = true;
		if let ContentItemType::Package { req: req2 } = &mut item.ty {
			*req2 = req.clone();
		}

		packages.insert(req);
	}

	let parent_packages =
		parent_configs
			.iter()
			.fold(TemplatePackageConfiguration::default(), |mut acc, cfg| {
				acc.merge(cfg.packages.clone());
				acc
			});

	let derived_global_packages = parent_packages.iter_global().map(|x| (x, None));
	let derived_client_packages = parent_packages
		.iter_side(Side::Client)
		.map(|x| (x, Some(Side::Client)));
	let derived_server_packages = parent_packages
		.iter_side(Side::Server)
		.map(|x| (x, Some(Side::Server)));
	for (package, _side) in derived_global_packages
		.chain(derived_client_packages)
		.chain(derived_server_packages)
	{
		let req = PkgRequest::parse(package.get_pkg_id(), PkgRequestSource::UserRequire).arc();
		if items
			.iter()
			.any(|x| matches!(&x.ty, ContentItemType::Package {req: req2} if *req2 == req))
		{
			continue;
		}

		let item = if let Some(pos) = items
			.iter()
			.position(|x| matches!(&x.ty, ContentItemType::Package {req: req2} if *req2 == req))
		{
			&mut items[pos]
		} else {
			items.push(ContentItem {
				ty: ContentItemType::Package { req: req.clone() },
				id: package.get_pkg_id(),
				is_configured: false,
				is_locked: false,
				is_derived: false,
				files_exist: false,
				locked_version: None,
				locked_addons: PtrEq(Arc::default()),
				locked_packages: PtrEq(Arc::default()),
				addon_ty: None,
			});
			items.last_mut().unwrap()
		};

		// Don't overwrite configured packages with derived ones
		if item.is_configured {
			continue;
		}

		item.is_configured = true;
		item.is_derived = true;

		if let ContentItemType::Package { req: req2 } = &mut item.ty {
			*req2 = req.clone();
		}

		packages.insert(req);
	}

	if let Some(addons) = addons {
		for addon in addons {
			// Only include "free" addons that aren't part of any package
			if items
				.iter()
				.any(|item| item.locked_addons.0.iter().any(|x| x.is_source(addon)))
			{
				continue;
			}

			items.push(ContentItem {
				ty: ContentItemType::Addon,
				id: addon.file_name.clone().into(),
				is_configured: false,
				is_locked: false,
				is_derived: false,
				files_exist: true,
				locked_version: None,
				locked_addons: PtrEq(Arc::default()),
				locked_packages: PtrEq(Arc::default()),
				addon_ty: Some(PackageKind::from_addon_kind(addon.kind)),
			});
		}
	}

	items.sort();

	(items, packages.into_iter().sorted().collect())
}

fn filter_sort_items(
	mut items: Vec<ContentItem>,
	info: &HashMap<ArcPkgReq, anyhow::Result<PkgInfo>>,
	filter: &Filter,
	search: &str,
	pkg_ty: Option<&PackageKind>,
) -> Vec<ContentItem> {
	let search = search.to_lowercase();

	items.retain(|x| {
		if *filter != Filter::All && x.ty == ContentItemType::Addon {
			return false;
		}
		if *filter == Filter::Dependencies && x.is_configured {
			return false;
		} else if *filter == Filter::Configured && !x.is_configured {
			return false;
		}

		if let Some(pkg_ty) = pkg_ty {
			if let Some(ty) = x.get_addon_ty(info) {
				if ty != *pkg_ty {
					return false;
				}
			} else {
				return false;
			}
		}

		if !search.is_empty() {
			let to_search = x.get_name(info);
			if !to_search.to_lowercase().contains(&search) {
				return false;
			}
		}

		true
	});

	#[derive(PartialEq, Eq, PartialOrd, Ord)]
	enum SortableType {
		Modpack,
		Package,
		Addon,
	}

	#[derive(PartialEq, Eq, PartialOrd, Ord)]
	struct Sort {
		ty: SortableType,
		name: String,
	}

	items.sort_by_key(|x| {
		let ty = match &x.ty {
			ContentItemType::Modpack { .. } => SortableType::Modpack,
			ContentItemType::Package { .. } => SortableType::Package,
			ContentItemType::Addon => SortableType::Addon,
		};

		Sort {
			ty,
			name: x.get_name(info).to_string(),
		}
	});

	items
}

fn badge(ico: &str, color: impl Into<Color>, theme: &Theme) -> Rect {
	let color = color.into();
	rect()
		.corner_radius(theme.round)
		.color(color)
		.padding(5.0)
		.child(icon(ico, 16.0))
}
