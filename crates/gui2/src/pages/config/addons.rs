use std::{
	collections::{HashMap, HashSet},
	rc::Rc,
	sync::Arc,
};

use itertools::Itertools;
use nitrolaunch::{
	config_crate::{
		ConfigKind,
		template::{TemplateConfig, TemplatePackageConfiguration},
	},
	instance_crate::lock::InstanceLockfile,
	pkg_crate::{PkgRequest, PkgRequestSource},
	shared::{Side, pkg::ArcPkgReq},
};

use crate::{
	components::input::{
		select::Selected,
		text::{TextInput, search_bar},
	},
	ops::{
		ConditionalQuery,
		packages::{FetchInstanceLockfile, FetchPackages, PkgInfo, PreloadPackages},
	},
	pages::config::ConfigState,
	prelude::*,
	util::PtrEq,
};

#[derive(PartialEq)]
pub struct AddonsConfig {
	pub config_state: ConfigState,
	pub parent_configs: PtrEq<[TemplateConfig]>,
}

impl Component for AddonsConfig {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let lockfile = use_query(ConditionalQuery::new(
			FetchInstanceLockfile::new(back_state.clone()),
			self.config_state.ty == ConfigKind::Instance,
			|| self.config_state.id.read().cloned(),
		));
		let filter = use_state(|| "all".to_string());
		let search = use_state(|| String::new());
		let side = use_state(|| "any".to_string());

		let modpack = self.config_state.modpack.clone();
		let packages = self.config_state.packages.clone();
		let results = use_memo(move || {
			build_items(
				lockfile.read().state().ok(),
				modpack.read().as_ref(),
				&*packages.read(),
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
			FetchPackages::new(back_state),
			preload.read().state().is_ok(),
			move || results.read().1.clone(),
		));
		let default_packages = Arc::new(HashMap::new());

		let packages = use_memo(move || {
			unsafe {
				(*((&packages as *const _) as *const State<Query<ConditionalQuery<FetchPackages>>>))
					.read()
			};
			// packages.query.read();
			// println!("Packages {:?}", packages.peek().state());
			let packages = packages.read();
			let packages = packages.state();
			PtrEq(packages.ok().unwrap_or(&default_packages).clone())
		});

		let packages2 = packages.clone();
		let processed_items = use_memo(move || {
			filter_sort_items(
				results.read().0.clone(),
				&packages2.read().0,
				&*filter.read(),
				&*search.read(),
			)
		});

		let items =
			VirtualScrollView::new_with_data(packages.read().cloned(), move |i, packages| {
				let reading = processed_items.peek();
				let item = reading.get(i).unwrap();

				ContentItemElem {
					item: item.clone(),
					packages: packages.0.clone(),
				}
				.into_element()
			})
			.expanded()
			// Conservative
			.item_size(64.0 + theme.gap)
			.length(processed_items.read().len());

		let on_select_filter =
			Rc::new(move |new_filter: Selected| filter.clone().set(new_filter.single()));
		let filters = Dropdown::new(Selected::Single(filter.read().clone()), on_select_filter)
			.child(SelectOption::new("all", "All", Some("box")))
			.child(SelectOption::new(
				"dependencies",
				"Dependencies",
				Some("diagram"),
			));

		let search_input = search_bar(TextInput::new(search), &theme);

		let on_select_side = Rc::new(move |new_side: Selected| side.clone().set(new_side.single()));
		let sides = Dropdown::new(Selected::Single(side.read().clone()), on_select_side)
			.child(SelectOption::new("any", "Any Side", Some("box")))
			.child(SelectOption::new("client", "Client", Some("controller")))
			.child(SelectOption::new("server", "Server", Some("server")));

		let controls = rect().width(Size::fill()).cont();
		let filters = rect()
			.width(Size::fill())
			.cont()
			.child(
				rect()
					.width(Size::flex(1.0))
					.center()
					.main_align(Alignment::Start)
					.child(filters),
			)
			.child(rect().width(Size::flex(2.0)).child(search_input))
			.child(
				rect()
					.width(Size::flex(1.0))
					.center()
					.main_align(Alignment::End)
					.child(sides),
			);
		let header = rect()
			.width(Size::fill())
			.spacing(theme.gap)
			.child(controls)
			.child(filters);

		rect()
			.expanded()
			.spacing(theme.gap)
			.padding(theme.gap)
			.child(header)
			.child(items)
	}
}

struct ContentItemElem {
	item: ContentItem,
	packages: Arc<HashMap<ArcPkgReq, anyhow::Result<PkgInfo>>>,
}

impl PartialEq for ContentItemElem {
	fn eq(&self, other: &Self) -> bool {
		self.item == other.item && Arc::ptr_eq(&self.packages, &other.packages)
	}
}

impl Component for ContentItemElem {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);

		let req = match &self.item {
			ContentItem::Package { req, .. } | ContentItem::Modpack { req, .. } => Some(req),
		};
		let info = req.and_then(|req| self.packages.get(req).and_then(|x| x.as_ref().ok()));

		let default_icon = icon("box", 32.0).into_element();

		let (ico, name, id, badges) = match &self.item {
			ContentItem::Package {
				req,
				is_configured,
				is_locked,
			}
			| ContentItem::Modpack {
				req,
				is_configured,
				is_locked,
			} => {
				let mut badges = Vec::new();
				if *is_configured && !*is_locked {
					badges.push(badge("warning", theme.warning, &theme).into_element());
				}
				if !*is_configured && *is_locked {
					badges.push(badge("diagram", theme.fg3, &theme).into_element());
				}
				if let Some(info) = info {
					let name = info.meta.name.clone().unwrap_or(req.to_string());
					let ico = info
						.meta
						.icon
						.as_deref()
						.map(|x| {
							ImageViewer::new(x.parse::<Uri>().unwrap_or_default())
								.width(Size::px(40.0))
								.height(Size::px(40.0))
								.corner_radius(theme.round)
								.into_element()
						})
						.unwrap_or(default_icon);

					(ico, name.clone(), req.to_string(), badges)
				} else {
					if let Some(Err(err)) = self.packages.get(req) {
						(
							icon("error", 24.0).color(theme.error).into_element(),
							err.root_cause().to_string(),
							req.to_string(),
							badges,
						)
					} else {
						(
							CircularLoader::new().into_element(),
							"Loading".into(),
							req.to_string(),
							badges,
						)
					}
				}
			}
		};

		let height = if let ContentItem::Modpack { .. } = &self.item {
			Size::px(76.0)
		} else {
			Size::px(64.0)
		};

		rect()
			.width(Size::fill())
			.height(height)
			.cont()
			.panel_colorway(&theme, *is_hovered.read(), false)
			.corner_radius(theme.round2)
			.hover(is_hovered)
			.margin(Gaps::new(0.0, 0.0, theme.gap, 0.0))
			.child(
				rect()
					.width(Size::px(64.0))
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
					.child(label().text(id).color(theme.fg3)),
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
			)
	}
}

#[derive(PartialEq, Clone, PartialOrd, Eq, Ord, Hash, Debug)]
enum ContentItem {
	Modpack {
		req: ArcPkgReq,
		is_configured: bool,
		is_locked: bool,
	},
	Package {
		req: ArcPkgReq,
		is_configured: bool,
		is_locked: bool,
	},
}

fn build_items(
	lockfile: Option<&InstanceLockfile>,
	modpack: Option<&String>,
	configured_packages: &TemplatePackageConfiguration,
) -> (Vec<ContentItem>, Vec<ArcPkgReq>) {
	let mut items = Vec::new();
	let mut packages = HashSet::new();

	if let Some(modpack) = modpack {
		let lock_modpack = lockfile.and_then(|x| x.get_modpack());
		let req = PkgRequest::parse(modpack, PkgRequestSource::UserRequire).arc();
		items.push(ContentItem::Modpack {
			req: req.clone(),
			is_configured: true,
			is_locked: lock_modpack.is_some(),
		});
		packages.insert(req);
	}

	if let Some(lockfile) = &lockfile {
		for (pkg, _) in lockfile.get_packages() {
			let req = PkgRequest::parse(pkg, PkgRequestSource::UserRequire).arc();
			items.push(ContentItem::Package {
				req: req.clone(),
				is_locked: true,
				is_configured: false,
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
			.position(|x| matches!(x, ContentItem::Package { req: req2, .. } if req2 == &req))
		{
			&mut items[pos]
		} else {
			items.push(ContentItem::Package {
				req: req.clone(),
				is_configured: true,
				is_locked: false,
			});
			items.last_mut().unwrap()
		};
		if let ContentItem::Package { is_configured, .. } = item {
			*is_configured = true;
		}

		packages.insert(req);
	}

	items.sort();

	(items, packages.into_iter().sorted().collect())
}

fn filter_sort_items(
	mut items: Vec<ContentItem>,
	info: &HashMap<ArcPkgReq, anyhow::Result<PkgInfo>>,
	filter: &str,
	search: &str,
) -> Vec<ContentItem> {
	let search = search.to_lowercase();

	items.retain(|x| {
		let (ContentItem::Modpack {
			is_configured,
			is_locked,
			req,
		}
		| ContentItem::Package {
			is_configured,
			is_locked,
			req,
		}) = x;
		if filter == "dependencies" && !*is_configured && *is_locked {
			return false;
		}

		if !search.is_empty() {
			if let Some(Ok(info)) = info.get(req) {
				if !info
					.meta
					.name
					.as_ref()
					.is_some_and(|x| x.to_lowercase().contains(&search))
				{
					return false;
				}
			}
		}

		true
	});

	items.sort_by_key(|x| {
		let (ContentItem::Modpack { req, .. } | ContentItem::Package { req, .. }) = x;
		if let Some(Ok(info)) = info.get(req) {
			info.meta.name.clone().unwrap_or_else(|| req.to_string())
		} else {
			req.to_string()
		}
	});

	items
}

fn badge(ico: &str, color: impl Into<Color>, theme: &Theme) -> impl IntoElement {
	let color = color.into();
	rect()
		.corner_radius(theme.round)
		.color(color)
		.border(theme.border(color))
		.padding(5.0)
		.child(icon(ico, 16.0))
}
