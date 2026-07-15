use std::rc::Rc;

use nitrolaunch::{
	pkg::search::{PackageMultiSearchResults, PackageSearchSession},
	pkg_crate::PackageMetaAndProps,
	shared::{
		loaders::Loader,
		pkg::{ArcPkgReq, PackageCategory, PackageKind, PackageSearchParameters},
	},
};

use crate::{
	components::{
		footer::FooterItem,
		input::{
			select::Selected,
			text::{TextInput, search_bar},
		},
		nav::page_buttons::PageButtons,
		pkg::RepoSelector,
		tag::repo_tag,
	},
	ops::packages::{SearchPackages, SearchPackagesParams},
	pages::package::view::PackageView,
	prelude::*,
	util::assets::get_package_kind_icon,
};

const PAGE_SIZE: u8 = 16;

#[derive(PartialEq)]
pub struct BrowsePackagesPage;

impl Component for BrowsePackagesPage {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();
		let search_session = use_state(|| PackageSearchSession::new(PAGE_SIZE));

		let search_state = PackageSearchState::new(&back_state);
		let search_state2 = search_state.clone();
		let search = use_memo(move || search_state2.to_search_params());

		let search_state2 = search_state.clone();
		let results_query = use_query(Query::new(
			SearchPackagesParams {
				search: search.read().cloned(),
				session: Captured(search_session.peek().cloned()),
				repo: search_state2.repo.read().cloned(),
			},
			SearchPackages::new(back_state.clone()),
		));
		let results = use_state(|| PackageMultiSearchResults {
			results: Vec::new(),
			total_results: 0,
		});

		let results_query2 = results_query.clone();
		let mut search_session2 = search_session.clone();
		let mut results2 = results.clone();
		use_side_effect(move || {
			unsafe {
				(*((&results_query as *const _) as *const State<Query<SearchPackages>>)).read()
			};

			if let Some(result) = results_query2.read().state().ok() {
				results2.set(result.0.clone());
				search_session2.set(result.1.clone());
			}
		});

		let selected_pkg = use_state::<Option<ArcPkgReq>>(|| None);
		use_side_effect(move || {
			if let Some(req) = &*selected_pkg.read() {
				front_state
					.write()
					.set_footer(FooterItem::InstallPackage(req.clone()));
			} else {
				front_state.write().set_footer(FooterItem::None);
			}
		});

		let top_upper_bar = rect()
			.width(Size::fill())
			.height(Size::percent(50.0))
			.cont()
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.padding(theme.gap)
					.center()
					.child(RepoSelector {
						repo: search_state.repo.clone(),
					}),
			)
			.child(rect().width(Size::flex(1.0)).height(Size::fill()))
			.child(rect().width(Size::flex(1.0)).height(Size::fill()));

		let mut page2 = search_state.page.clone();
		let page_buttons = PageButtons {
			page: (*search_state.page.read()).into(),
			total_pages: results.read().total_results,
			on_set: (move |new_page| page2.set(new_page as u16)).into(),
		};
		let search = TextInput::new(search_state.search.clone());

		let pkg_ty = search_state.ty.clone();
		let ty_selector = Dropdown::new(
			Selected::Single(search_state.ty.read().clone()),
			Rc::new(move |selected| {
				pkg_ty.clone().set(selected.single());
			}),
		)
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

		let top_lower_bar = rect()
			.width(Size::fill())
			.height(Size::percent(50.0))
			.cont()
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.padding(theme.gap)
					.center()
					.child(ty_selector),
			)
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.center()
					.child(page_buttons),
			)
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.padding(theme.gap)
					.center()
					.child(search_bar(search, &theme)),
			);

		let top_bar = rect()
			.width(Size::fill())
			.height(Size::px(100.0))
			.border(border_bottom(theme.border, theme.panel_border))
			.child(top_upper_bar)
			.child(top_lower_bar);

		let is_loading = !results_query.read().state().is_ok();
		let packages = results.read();
		let packages_view = ScrollView::new().expanded().spacing(theme.gap);
		let packages = if is_loading {
			packages_view.children(
				(0..PAGE_SIZE)
					.map(|_| skeleton(Size::fill(), Size::px(64.0), &theme).into_element()),
			)
		} else {
			packages_view.children(packages.results.iter().map(|req| {
				let preview = search_session.peek().previews().get(req).map(|x| x.clone());

				BrowseItem {
					req: req.clone(),
					preview,
					selected_package: selected_pkg.clone(),
				}
				.into_element()
			}))
		};
		let left_bar = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.vertical()
			.padding(theme.gap)
			.border(border_right(theme.border, theme.panel_border))
			.child(packages);

		let preview = rect()
			.width(Size::flex(3.5))
			.height(Size::fill())
			.maybe(selected_pkg.read().is_some(), |this| {
				this.child(PackageView {
					req: selected_pkg.peek().cloned().unwrap(),
				})
			})
			.maybe(selected_pkg.read().is_none(), |this| {
				this.child(placeholder("Select a package", &theme))
			});

		rect().expanded().child(top_bar).child(
			rect()
				.width(Size::fill())
				.height(Size::flex(1.0))
				.horizontal()
				.flex()
				.child(left_bar)
				.child(preview),
		)
	}
}

#[derive(Clone)]
struct PackageSearchState {
	repo: State<Option<String>>,
	page: State<u16>,
	search: State<String>,
	ty: State<PackageKind>,
	categories: State<Vec<PackageCategory>>,
	mc_versions: State<Vec<String>>,
	loaders: State<Vec<Loader>>,
}

impl PackageSearchState {
	fn new(back_state: &BackState) -> Self {
		let out = Self {
			repo: use_state(|| None),
			page: use_state(|| 0),
			search: use_state(|| String::new()),
			ty: use_state(|| PackageKind::Mod),
			categories: use_state(|| Vec::new()),
			mc_versions: use_state(|| Vec::new()),
			loaders: use_state(|| Vec::new()),
		};

		let mut state2 = out.clone();
		let back_state = back_state.clone();
		use_side_effect(move || {
			state2.repo.read();
			state2.search.read();
			state2.ty.read();
			state2.categories.read();
			state2.mc_versions.read();
			state2.loaders.read();

			state2.page.set_if_modified(0);

			// Handle what package types and categories are available based on the selected repository
			if let Some(repo) = &*state2.repo.peek() {
				if let Some(repo) = back_state.repos().get(repo) {
					if !repo.package_types.is_empty()
						&& !repo.package_types.contains(&*state2.ty.peek())
					{
						state2.ty.set(repo.package_types[0]);
					}

					if !repo.package_categories.is_empty()
						&& !state2
							.categories
							.peek()
							.iter()
							.all(|x| repo.package_categories.contains(x))
					{
						state2.categories.set(Vec::new());
					}
				}
			}
		});

		out
	}

	fn to_search_params(&self) -> PackageSearchParameters {
		PackageSearchParameters {
			count: PAGE_SIZE,
			skip: *self.page.read() as usize * PAGE_SIZE as usize,
			search: Some(self.search.read().clone()).filter(|x| !x.is_empty()),
			types: vec![*self.ty.read()],
			minecraft_versions: self.mc_versions.read().cloned(),
			loaders: self.loaders.read().cloned(),
			categories: self.categories.read().cloned(),
		}
	}
}

#[derive(PartialEq)]
struct BrowseItem {
	req: ArcPkgReq,
	preview: Option<PackageMetaAndProps>,
	selected_package: State<Option<ArcPkgReq>>,
}

impl Component for BrowseItem {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let is_hovered = use_state(|| false);

		let is_selected = self
			.selected_package
			.read()
			.as_ref()
			.is_some_and(|x| x == &self.req);
		let bg = if is_selected {
			theme.item_select
		} else if *is_hovered.read() {
			theme.panel_hover
		} else {
			theme.bg
		};

		let meta = self.preview.as_ref().map(|x| &x.meta);
		let props = self.preview.as_ref().map(|x| &x.props);

		let name = meta.and_then(|x| x.name.as_deref()).unwrap_or(&self.req.id);

		let default_icon = icon("box", 32.0).into_element();
		let ico = meta
			.and_then(|x| x.icon.as_ref())
			.map(|x| {
				let default_icon = default_icon.clone();
				img(x)
					.error_renderer(move |_| default_icon.clone())
					.width(Size::px(40.0))
					.height(Size::px(40.0))
					.corner_radius(theme.round)
					.into_element()
			})
			.unwrap_or(default_icon);

		let description = meta.and_then(|x| x.description.as_deref()).map(|x| {
			rect()
				.width(Size::flex(1.0))
				.child(clip_text(x).color(theme.fg3).font_size(12.0))
		});

		let req = self.req.clone();
		let mut selected_package = self.selected_package.clone();

		let repo = self.req.repository.as_deref().map(|x| {
			rect()
				.cross_align(Alignment::End)
				.margin(Gaps::new(0.0, theme.gap2 * 2.0, 0.0, 0.0))
				.child(repo_tag(x, true, &back_state, &theme))
		});
		let upper_details = rect()
			.width(Size::fill())
			.cont()
			.cross_align(Alignment::Center)
			.child(segment(name, 1.0))
			.maybe_child(repo);
		let lower_details = rect()
			.width(Size::fill())
			.cont()
			.spacing(theme.gap2)
			.cross_align(Alignment::Center)
			.maybe_child(description);
		let details = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.center()
			.spacing(theme.gap)
			.cross_align(Alignment::Start)
			.child(upper_details)
			.child(lower_details);

		rect()
			.width(Size::fill())
			.height(Size::px(64.0))
			.hover(is_hovered)
			.corner_radius(theme.round)
			.background(bg)
			.cont()
			.spacing(0.0)
			.on_press(move |_| {
				selected_package.set_if_modified(Some(req.clone()));
			})
			.child(
				rect()
					.width(Size::px(64.0))
					.height(Size::fill())
					.center()
					.child(ico),
			)
			.child(details)
	}
}
