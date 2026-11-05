use std::rc::Rc;

use itertools::Itertools;
use nitrolaunch::shared::servers::{ServerCategory, ServerInfo, ServerSearchParams};

use crate::{
	components::{
		input::{select::Selected, text::{TextInput, search_bar}},
		nav::page_buttons::PageButtons,
	},
	ops::{servers::{SearchServers, ServerSearchResults}, versions::FetchMinecraftVersions},
	prelude::*,
};

const PAGE_SIZE: u8 = 16;

#[derive(PartialEq)]
pub struct ServersPage;

impl Component for ServersPage {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let search_state = ServerSearchState::new();
		let search_state2 = search_state.clone();
		let search = use_memo(move || search_state2.to_search_params());
		let results_query = use_query(Query::new(
			search.read().cloned(),
			SearchServers::new(back_state.clone()),
		));
		let results = use_state(ServerSearchResults::default);

		let results_query2 = results_query;
		let mut results2 = results;
		use_side_effect(move || {
			if let Some(result) = results_query2.read().state().ok() {
				results2.set(result.clone());
			}
		});

		if back_state.server_repos().is_empty() {
			return placeholder(
				"No server repositories available from plugins. Please install a plugin that provides server repositories.",
				&theme,
			)
			.font_size(18.0);
		}

		let mut page = search_state.page;
		let page_buttons = PageButtons {
			page: (*search_state.page.read()).into(),
			total_pages: results.read().total_results.div_ceil(PAGE_SIZE as usize),
			on_set: (move |new_page| page.set(new_page as u16)).into(),
		};

		let is_loading = results_query.read().state().is_loading()
			|| results_query.read().state().is_pending();
		let servers = results.read();
		let servers_view = ScrollView::new().expanded().spacing(theme.gap);
		let servers = if is_loading {
			servers_view.children(
				(0..PAGE_SIZE).map(|_| skeleton(Size::fill(), Size::px(64.0), &theme).into_element()),
			)
		} else if servers.results.is_empty() {
			servers_view.child(placeholder("No results found", &theme))
		} else {
			servers_view.children(servers.results.iter().map(|server| {
				ServerItem {
					server: server.clone(),
					selected_server: search_state.selected,
				}
				.into_element()
			}))
		};

		let top_bar = rect()
			.width(Size::fill())
			.height(Size::px(theme.input_height + theme.gap2 * 2.0))
			.padding(Gaps::new(theme.gap2, theme.gap2, theme.gap2, 0.0))
			.cont()
			.border(border_bottom(theme.border, theme.panel_border))
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.center()
					.child(page_buttons),
			)
			.child(ServerFilters {
				state: search_state.clone(),
			});

		let left_bar = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.vertical()
			.padding(theme.gap)
			.border(border_right(theme.border, theme.panel_border))
			.child(servers);

		let preview = search_state.selected.read().clone().map(|server| {
			ServerPreview { server }.into_element()
		});
		let preview = rect()
			.width(Size::flex(3.5))
			.height(Size::fill())
			.maybe_child(preview)
			.maybe(search_state.selected.read().is_none(), |this| {
				this.child(placeholder("Select a server", &theme))
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

#[derive(Clone, PartialEq)]
struct ServerSearchState {
	repository: State<Option<String>>,
	selected: State<Option<ServerInfo>>,
	page: State<u16>,
	search: State<String>,
	minecraft_version: State<Option<String>>,
	categories: State<Vec<ServerCategory>>,
}

impl ServerSearchState {
	fn new() -> Self {
		let out = Self {
			repository: use_state(|| None),
			selected: use_state(|| None),
			page: use_state(|| 0),
			search: use_state(String::new),
			minecraft_version: use_state(|| None),
			categories: use_state(Vec::new),
		};

		let mut state = out.clone();
		use_side_effect(move || {
			state.repository.read();
			state.search.read();
			state.minecraft_version.read();
			state.categories.read();
			state.page.set_if_modified(0);
			state.selected.set_if_modified(None);
		});

		out
	}

	fn to_search_params(&self) -> ServerSearchParams {
		ServerSearchParams {
			repository: self.repository.read().clone(),
			count: PAGE_SIZE,
			skip: *self.page.read() as usize * PAGE_SIZE as usize,
			search: Some(self.search.read().clone()).filter(|x| !x.is_empty()),
			minecraft_version: self.minecraft_version.read().clone(),
			categories: self.categories.read().clone(),
		}
	}
}

#[derive(PartialEq)]
struct ServerFilters {
	state: ServerSearchState,
}

impl Component for ServerFilters {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let search = TextInput::new(self.state.search);

		let repository = self.state.repository;
		let repository_selector = Dropdown::new(
			Selected::Single(repository.read().clone()),
			Rc::new(move |selected| repository.clone().set(selected.single_optional().flatten())),
		)
		.panel_colorway()
		.header_width(Size::px(180.0))
		.allow_none()
		.children(
			back_state
				.server_repos()
				.iter()
				.map(|repo| SelectOption::simple_or_none(Some(repo.id.clone()))),
		);

		let version = self.state.minecraft_version;
		let versions = use_query(FetchMinecraftVersions::new(back_state, false));
		let version_options = {
			let versions_read = versions.read();
			versions_read
				.state()
				.ok()
				.into_iter()
				.flatten()
				.rev()
				.map(|version| SelectOption::simple_or_none(Some(version.clone())))
				.collect::<Vec<_>>()
		};
		let version_selector = Dropdown::new(
			Selected::Single(version.read().clone()),
			Rc::new(move |selected| version.clone().set(selected.single_optional().flatten())),
		)
		.panel_colorway()
		.header_width(Size::px(140.0))
		.allow_none()
		.children(version_options);

		let categories = self.state.categories;
		let category_selector = Dropdown::new(
			Selected::Multi(categories.read().clone()),
			Rc::new(move |selected| categories.clone().set(selected.multi())),
		)
		.panel_colorway()
		.children(server_categories().into_iter().map(|category| {
			SelectOption::new(category, category_name(category), Some("tag"))
		}));

		rect()
			.width(Size::flex(3.5))
			.height(Size::fill())
			.cont()
			.cross_align(Alignment::Center)
			.child(repository_selector)
			.child(version_selector)
			.child(category_selector)
			.child(rect().width(Size::flex(1.0)).child(search_bar(search, &theme)))
	}
}

#[derive(PartialEq)]
struct ServerItem {
	server: ServerInfo,
	selected_server: State<Option<ServerInfo>>,
}

impl Component for ServerItem {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);
		let is_selected = self.selected_server.read().as_ref() == Some(&self.server);
		let server = self.server.clone();
		let mut selected_server = self.selected_server;
		let default_icon = icon("server", 32.0).into_element();
		let server_icon = self.server.icon.as_ref().map(|source| {
			let default_icon = default_icon.clone();
			img(source)
				.error_renderer(move |_| default_icon.clone())
				.width(Size::px(40.0))
				.height(Size::px(40.0))
				.corner_radius(theme.round)
				.into_element()
		}).unwrap_or(default_icon);

		rect()
			.width(Size::fill())
			.height(Size::px(64.0))
			.hover(is_hovered)
			.clickable()
			.corner_radius(theme.round)
			.simple_colorway(&theme, *is_hovered.read(), is_selected)
			.cont()
			.child(rect().width(Size::px(64.0)).height(Size::fill()).center().child(server_icon))
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.center()
					.spacing(theme.gap2)
					.child(segment(self.server.name.clone(), 1.0))
					.child(clip_text(&self.server.address).color(theme.fg3).font_size(12.0)),
			)
			.maybe_child(self.server.online_players.map(|players| {
				rect().margin(Gaps::new(0.0, theme.gap2, 0.0, 0.0)).child(
					clip_text(&format!("{players} online")).color(theme.fg3).font_size(12.0),
				)
			}))
			.on_press(move |_| selected_server.set_if_modified(Some(server.clone())))
	}
}

#[derive(PartialEq)]
struct ServerPreview {
	server: ServerInfo,
}

impl Component for ServerPreview {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let server = &self.server;
		let icon = server.icon.as_ref().map(|source| {
			img(source)
				.width(Size::px(72.0))
				.height(Size::px(72.0))
				.corner_radius(theme.round2)
			.into_element()
		}).unwrap_or_else(|| icon("server", 56.0).into_element());
		let categories = server.categories.iter().map(|category| category_name(*category)).join(", ");
		let versions = server.minecraft_versions.join(", ");
		let loaders = server.loaders.iter().map(ToString::to_string).join(", ");

		rect().expanded().padding(theme.gap3).child(
			ScrollView::new().expanded().child(
				rect()
					.width(Size::fill())
					.spacing(theme.gap3)
					.child(rect().center().child(icon))
					.child(label().text(server.name.clone()).font_size(theme.font2).font_weight(FontWeight::BOLD))
					.child(label().text(server.address.clone()).color(theme.fg2))
					.child(clip_text(&server.description).color(theme.fg2))
					.child(label().text(format!("Categories: {categories}")))
					.child(label().text(format!("Minecraft: {versions}")))
					.child(label().text(format!("Loaders: {loaders}")))
					.maybe_child(server.online_players.map(|players| label().text(format!("{players} players online"))))
					.maybe_child(server.is_application.then(|| label().text("Application link")))
			),
		)
	}
}

fn server_categories() -> [ServerCategory; 10] {
	[
		ServerCategory::Pvp,
		ServerCategory::Pve,
		ServerCategory::Survival,
		ServerCategory::Creative,
		ServerCategory::Minigames,
		ServerCategory::Modded,
		ServerCategory::Economy,
		ServerCategory::Skyblock,
		ServerCategory::Anarchy,
		ServerCategory::Other,
	]
}

fn category_name(category: ServerCategory) -> &'static str {
	match category {
		ServerCategory::Pvp => "PvP",
		ServerCategory::Pve => "PvE",
		ServerCategory::Survival => "Survival",
		ServerCategory::Creative => "Creative",
		ServerCategory::Minigames => "Minigames",
		ServerCategory::Modded => "Modded",
		ServerCategory::Economy => "Economy",
		ServerCategory::Skyblock => "Skyblock",
		ServerCategory::Anarchy => "Anarchy",
		ServerCategory::Other => "Other",
	}
}
