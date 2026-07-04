use nitrolaunch::shared::pkg::ArcPkgReq;

use crate::{components::tag::repo_tag, ops::packages::FetchPackageDetails, prelude::*};

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
		let mut tab = use_state(|| Tab::Description);

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
			ImageViewer::new(ico.parse::<Uri>().unwrap_or_default())
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
			.child(label().text(name).font_size(18));

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
			.child(details);

		let banner = meta.banner.as_ref().map(|x| {
			let image = ImageViewer::new(x.parse::<Uri>().unwrap_or_default())
				.expanded()
				.aspect_ratio(AspectRatio::Max)
				.opacity(0.25)
				.error_renderer(|_| rect().into_element());
			let gradient = rect()
				.expanded()
				.position(Position::new_absolute())
				.background_linear_gradient(
					LinearGradient::new()
						.stop((Color::TRANSPARENT, 0.0))
						.stop((theme.bg, 100.0)),
				);
			rect()
				.expanded()
				.position(Position::new_absolute())
				.child(image)
				.child(gradient)
		});

		let top_container = rect()
			.width(Size::fill())
			.height(Size::px(80.0))
			.border(border_bottom(theme.border, theme.panel_border))
			.maybe_child(banner)
			.child(top);

		let main = rect().width(Size::fill()).height(Size::flex(1.0));

		rect().expanded().child(top_container).child(main)
	}
}

#[derive(PartialEq)]
enum Tab {
	Description,
	Versions,
	Gallery,
}
