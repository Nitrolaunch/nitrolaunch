use nitrolaunch::config_crate::ConfigKind;

use crate::{
	components::{
		instance::running_instances::RunningInstances, output_indicator::OutputIndicator,
	},
	ops::{
		instance::InstanceItemInfo,
		launch::{LaunchInstance, LaunchInstanceParams},
	},
	pages::instance::config::ConfiguredItem,
	prelude::*,
};

#[derive(PartialEq)]
pub struct Footer;

impl Component for Footer {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let state = use_front_state();
		state.read().subscribe(FrontChannel::FooterItem);

		let left = rect()
			.height(Size::fill())
			.width(Size::flex(1.0))
			.child(RunningInstances);

		let center = rect()
			.height(Size::fill())
			.width(Size::flex(1.0))
			.child(FooterButton {
				item: state.read().footer().clone(),
			});

		let right = rect()
			.height(Size::fill())
			.width(Size::flex(1.0))
			.cont()
			.cross_align(Alignment::Center)
			.child(rect().width(Size::flex(2.0)).child(OutputIndicator))
			.child(rect().width(Size::flex(1.0)));

		rect()
			.width(Size::fill())
			.height(Size::px(theme.footer_height))
			.horizontal()
			.background(theme.footer)
			.flex()
			.child(left)
			.child(center)
			.child(right)
	}
}

#[derive(PartialEq)]
struct FooterButton {
	item: FooterItem,
}

impl Component for FooterButton {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();
		let launch_instance = use_mutation(LaunchInstance::new(back_state));

		let left = rect().height(Size::fill()).width(Size::flex(1.0));

		let (fg, border, bg) = if self.item == FooterItem::None {
			(theme.disabled, theme.disabled, theme.bg)
		} else {
			(theme.primary, theme.primary, theme.primary_bg)
		};

		let item = self.item.clone();
		let on_press = move |_| match &item {
			FooterItem::None => {}
			FooterItem::InstanceOrTemplate(info) => match info.ty {
				ConfigKind::Instance => {
					launch_instance.mutate(LaunchInstanceParams {
						id: info.id.clone(),
						account: None,
						offline: false,
					});
				}
				ConfigKind::Template | ConfigKind::BaseTemplate => {
					front_state
						.write()
						.set_configured_item(Some(ConfiguredItem {
							id: Some(info.id.clone()),
							ty: info.ty,
						}));
				}
			},
		};

		let center = rect()
			.height(Size::fill())
			.width(Size::px(128.0))
			.center()
			.child(
				button(&theme)
					.width(Size::fill())
					.height(Size::percent(75.0))
					.color(fg)
					.border_fill(border)
					.background(bg)
					.hover_background(bg)
					.on_press(on_press)
					.child(
						rect()
							.cont()
							.child(icon(self.item.icon(), 16.0))
							.child(self.item.title()),
					),
			);

		let right = rect().height(Size::fill()).width(Size::flex(1.0));

		rect()
			.width(Size::fill())
			.height(Size::px(theme.footer_height))
			.horizontal()
			.background(theme.footer)
			.flex()
			.child(left)
			.child(center)
			.child(right)
	}
}

/// What the footer has selected
#[derive(Clone, PartialEq)]
pub enum FooterItem {
	None,
	InstanceOrTemplate(InstanceItemInfo),
}

impl FooterItem {
	fn icon(&self) -> &'static str {
		match self {
			Self::None => "box",
			Self::InstanceOrTemplate(InstanceItemInfo {
				ty: ConfigKind::Instance,
				..
			}) => "play",
			Self::InstanceOrTemplate(InstanceItemInfo {
				ty: ConfigKind::Template | ConfigKind::BaseTemplate,
				..
			}) => "properties",
		}
	}

	fn title(&self) -> &'static str {
		match self {
			Self::None => "Select...",
			Self::InstanceOrTemplate(InstanceItemInfo {
				ty: ConfigKind::Instance,
				..
			}) => "Launch",
			Self::InstanceOrTemplate(InstanceItemInfo {
				ty: ConfigKind::Template | ConfigKind::BaseTemplate,
				..
			}) => "Edit",
		}
	}
}
