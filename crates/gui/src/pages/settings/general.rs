use std::{rc::Rc, sync::LazyLock};

use nitrolaunch::plugin_crate::hook::hooks::{Theme, ThemeType};

use crate::{
	components::input::slider, pages::settings::SettingsState, prelude::*, theme::ThemeDeser,
};

static BUILTIN_THEMES: LazyLock<[Theme; 2]> = LazyLock::new(|| {
	[
		Theme {
			id: "dark".into(),
			name: "Dark".into(),
			description: Some("Standard dark theme".into()),
			r#type: ThemeType::Base,
			css: String::new(),
			settings: serde_json::to_string(&ThemeDeser::dark()).unwrap_or_default(),
			color: "#1e1e1e".into(),
		},
		Theme {
			id: "light".into(),
			name: "Light".into(),
			description: Some("Standard light theme".into()),
			r#type: ThemeType::Base,
			css: String::new(),
			settings: serde_json::to_string(&ThemeDeser::light()).unwrap_or_default(),
			color: "#ffffff".into(),
		},
	]
});

#[derive(PartialEq)]
pub struct GeneralSettings {
	pub state: SettingsState,
}

impl Component for GeneralSettings {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();

		let base_themes = BUILTIN_THEMES
			.iter()
			.chain(
				back_state
					.themes()
					.iter()
					.filter(|x| matches!(x.r#type, ThemeType::Base) && !x.settings.is_empty()),
			)
			.map(|x| SelectableTheme {
				theme: NotEq(x.clone()),
				is_selected: *self.state.base_theme.read() == x.id,
				on_select: self.state.base_theme.setter(),
			});
		let base_themes = grid(3, base_themes).gap(theme.gap2);
		let base_themes = field("Base Theme", "palette", &theme, base_themes)
			.tip(&front_state, "The main theme to use for the launcher");

		let overlay_themes = back_state
			.themes()
			.iter()
			.filter(|x| matches!(x.r#type, ThemeType::Overlay) && !x.settings.is_empty())
			.map(|x| SelectableTheme {
				theme: NotEq(x.clone()),
				is_selected: self.state.overlay_themes.read().contains(&x.id),
				on_select: self.state.overlay_themes.select_setter(),
			});
		let overlay_themes = grid(3, overlay_themes).gap(theme.gap2);
		let overlay_themes = field("Overlay Themes", "palette", &theme, overlay_themes).tip(
			&front_state,
			"Additional effects to layer on top of the base theme. Use as many as you want.",
		);

		let zoom = slider(
			*self.state.zoom.read(),
			0.5,
			1.75,
			0.1,
			self.state.zoom.setter(),
			&theme,
			&front_state,
		);
		let zoom = field("Zoom Level", "search", &theme, zoom)
			.tip(&front_state, "Changes the scale of the entire launcher");

		let out = rect()
			.padding(theme.gap3)
			.child(base_themes)
			.child(overlay_themes)
			.child(zoom);

		ScrollView::new().expanded().child(out)
	}
}

#[derive(PartialEq)]
struct SelectableTheme {
	theme: NotEq<Theme>,
	is_selected: bool,
	on_select: EventHandler<String>,
}

impl Component for SelectableTheme {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);

		let calculated: Rc<crate::prelude::Theme> = use_hook(|| {
			Rc::new(
				serde_json::from_str::<ThemeDeser>(&self.theme.0.settings)
					.unwrap_or_else(|_| ThemeDeser::dark())
					.into(),
			)
		});

		let fg = if calculated.fg == Color::TRANSPARENT {
			theme.fg
		} else {
			calculated.fg
		};

		let preview = rect()
			.width(Size::fill())
			.height(Size::px(128.0))
			.background(calculated.bg)
			.color(fg)
			.corner_radius(theme.round)
			.main_align(Alignment::SpaceEvenly)
			.cross_align(Alignment::Center)
			.padding(theme.gap3)
			.spacing(theme.gap3)
			.child(
				rect()
					.width(Size::fill())
					.horizontal()
					.main_align(Alignment::SpaceEvenly)
					.cross_align(Alignment::Center)
					.child(swatch(calculated.item))
					.child(swatch(calculated.primary))
					.child(swatch(calculated.secondary)),
			)
			.maybe_child(self.theme.0.description.clone());

		let id = self.theme.0.id.clone();
		let on_select = self.on_select.clone();
		rect()
			.panel_colorway(&theme, *is_hovered.read(), self.is_selected)
			.corner_radius(theme.round)
			.padding(theme.gap3)
			.spacing(theme.gap3)
			.hover(is_hovered)
			.on_press(move |_| {
				on_select.call(id.clone());
			})
			.child(
				label()
					.text(self.theme.0.name.clone())
					.font_weight(FontWeight::BOLD),
			)
			.child(preview)
	}
}

fn swatch(color: Color) -> Rect {
	rect()
		.width(Size::px(12.0))
		.height(Size::px(12.0))
		.background(color)
		.corner_radius(6.0)
}
