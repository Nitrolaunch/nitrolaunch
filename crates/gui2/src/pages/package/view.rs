use itertools::Itertools;
use nitrolaunch::{pkg_crate::metadata::PackageMetadata, shared::pkg::ArcPkgReq};

use crate::{
	components::{input::tabs::TopTabs, pkg::versions::PackageVersions, tag::repo_tag},
	ops::packages::FetchPackageDetails,
	prelude::*,
	util::PtrEq,
};

#[derive(PartialEq)]
pub struct PackageView {
	pub req: ArcPkgReq,
}

impl Component for PackageView {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let details_query = use_query(Query::new(
			self.req.clone(),
			FetchPackageDetails::new(back_state.clone()).toast(
				&back_state,
				None,
				"Failed to fetch package",
			),
		));
		let tab = use_state(|| Tab::Description);

		let details = details_query.read();
		let details = details.state();
		let is_loading = !details.is_ok();
		let details = details.ok();
		let meta = details.map(|x| x.meta.clone()).unwrap_or_default();
		let props = details.map(|x| x.props.clone()).unwrap_or_default();

		let default_icon = icon("box", 48.0).into_element();
		let ico = if is_loading {
			CircularLoader::new().size(48.0).into_element()
		} else if let Some(ico) = &meta.icon {
			img(ico)
				.error_renderer(move |_| default_icon.clone())
				.width(Size::px(56.0))
				.height(Size::px(56.0))
				.corner_radius(theme.round2)
				.into_element()
		} else {
			default_icon
		};

		let name = if is_loading {
			"Loading".into()
		} else if let Some(name) = &meta.name {
			name.clone()
		} else {
			self.req.to_string()
		};

		let upper_details = rect()
			.width(Size::fill())
			.horizontal()
			.cross_align(Alignment::Center)
			.child(
				label()
					.text(name)
					.font_size(theme.font2)
					.font_weight(FontWeight::BOLD),
			);

		let repo = self
			.req
			.repository
			.as_deref()
			.map(|x| repo_tag(x, false, &back_state, &theme));
		let lower_details = rect()
			.width(Size::fill())
			.horizontal()
			.cross_align(Alignment::Center)
			.maybe_child(repo);

		let details = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.spacing(theme.gap2)
			.center()
			.child(upper_details)
			.child(lower_details);

		let description = rect()
			.width(Size::flex(2.0))
			.height(Size::fill())
			.main_align(Alignment::Center)
			.cross_align(Alignment::End)
			.padding(Gaps::new(0.0, theme.gap2 * 2.0, 0.0, 0.0))
			.child(
				clip_text(meta.description.as_deref().unwrap_or("..."))
					.color(theme.fg2)
					.text_align(TextAlign::End)
					.max_lines(2),
			);

		let top = rect()
			.expanded()
			.cont()
			.child(
				rect()
					.width(Size::px(80.0))
					.height(Size::fill())
					.center()
					.child(ico),
			)
			.child(details)
			.child(description);

		let banner = meta.banner.as_ref().map(|x| {
			let image = img(x)
				.expanded()
				.aspect_ratio(AspectRatio::Max)
				.opacity(0.25)
				.error_renderer(|_| rect().into_element())
				.loading_placeholder(rect());

			rect()
				.width(Size::fill())
				.height(Size::percent(100.0))
				.position(Position::new_absolute())
				.child(image)
		});

		let top_container = rect()
			.width(Size::fill())
			.height(Size::px(80.0))
			.border(border_bottom(theme.border, theme.panel_border))
			.maybe_child(banner)
			.child(top);

		let loading_spinner = rect()
			.expanded()
			.center()
			.child(CircularLoader::new())
			.into_element();
		let contents = match &*tab.read() {
			Tab::Description => {
				if let Some(long_description) = &meta.long_description {
					println!("{long_description}");
					let markdown = MarkdownViewer::new(long_description.clone())
						.width(Size::fill())
						.paragraph_size(14.0)
						.padding(32.0)
						.color(theme.fg)
						.code_font_size(14.0)
						.color_code(theme.fg)
						.background_code(theme.item);
					let markdown = ScrollView::new()
						.expanded()
						.direction(Direction::Vertical)
						.child(markdown);

					rect().expanded().child(markdown).into_element()
				} else if is_loading {
					loading_spinner
				} else {
					placeholder("No description provided", &theme).into_element()
				}
			}
			Tab::Versions => PackageVersions {
				req: self.req.clone(),
				meta: PtrEq(meta.clone()),
				props: PtrEq(props.clone()),
			}
			.into_element(),
			Tab::Gallery => {
				if let Some(gallery) = meta.gallery.as_ref().filter(|x| !x.is_empty()) {
					let items = gallery.iter().map(|x| {
						rect()
							.width(Size::fill())
							.height(Size::px(180.0))
							.corner_radius(theme.round2)
							.shiny_border(&theme)
							.child(
								img(x)
									.expanded()
									.aspect_ratio(AspectRatio::Max)
									.image_cover(ImageCover::Center)
									.corner_radius(theme.round2),
							)
					});
					let grid = grid(3, items).gap(theme.gap2);
					ScrollView::new().expanded().child(grid).into_element()
				} else if is_loading {
					loading_spinner
				} else {
					placeholder("Gallery empty", &theme).into_element()
				}
			}
		};
		let main = rect()
			.width(Size::fill())
			.height(Size::flex(1.0))
			.child(contents);

		let tabs = TopTabs::new(tab)
			.child(SelectOption::new(
				Tab::Description,
				"Description",
				Some("text"),
			))
			.child(SelectOption::new(Tab::Versions, "Versions", Some("tag")))
			.child(SelectOption::new(Tab::Gallery, "Gallery", Some("picture")));

		let main = rect()
			.width(Size::flex(4.0))
			.height(Size::fill())
			.flex()
			.child(tabs)
			.child(main);

		let right = rect()
			.width(Size::flex(1.5))
			.height(Size::fill())
			.border(border_left(theme.border, theme.panel_border))
			.child(properties(&self.req, &meta, &theme));

		let bottom = rect()
			.width(Size::fill())
			.height(Size::flex(1.0))
			.flex()
			.horizontal()
			.child(main)
			.child(right);

		rect().expanded().flex().child(top_container).child(bottom)
	}
}

#[derive(PartialEq, Clone)]
enum Tab {
	Description,
	Versions,
	Gallery,
}

fn properties(req: &ArcPkgReq, meta: &PackageMetadata, theme: &Theme) -> Rect {
	rect()
		.expanded()
		.padding(theme.gap)
		.spacing(theme.gap)
		.child(property("hashtag", "ID", req, theme))
		.maybe(meta.authors.is_some(), |this| {
			this.child(property(
				"user",
				"Authors",
				meta.authors.as_ref().unwrap().iter().join("  "),
				theme,
			))
		})
}

fn property(ico: &'static str, title: &str, value: impl ToString, theme: &Theme) -> Rect {
	rect()
		.width(Size::fill())
		.height(Size::px(32.0))
		.corner_radius(theme.round)
		.cont()
		.cross_align(Alignment::Center)
		.padding(Gaps::new(0.0, (32.0 - 16.0) / 2.0, 0.0, 0.0))
		.child(
			rect()
				.width(Size::px(32.0))
				.height(Size::fill())
				.center()
				.child(icon(ico, 16.0)),
		)
		.child(
			rect()
				.margin(Gaps::new(0.0, theme.gap, 0.0, 0.0))
				.child(title),
		)
		.child(
			segment(clip_text(&value.to_string()).color(theme.fg2), 1.0)
				.cross_align(Alignment::End)
				.overflow(Overflow::Clip),
		)
}
