use std::rc::Rc;

use nitrolaunch::{
	config_crate::ConfigKind, plugin_crate::hook::hooks::DropdownButtonLocation,
	shared::pkg::ArcPkgReq,
};

use crate::{
	components::{
		input::select::{Selected, run_dropdown_button},
		instance::running_instances::RunningInstances,
		output_indicator::OutputIndicator,
		pkg::install::PackageInstallModal,
	},
	ops::{
		instance::InstanceItemInfo,
		launch::{
			FetchInstanceRunState, InstanceRunState, KillInstance, LaunchInstance,
			LaunchInstanceParams,
		},
		packages::FetchPackageDetails,
		plugin_results::{FetchDropdownButtons, OpenCustomPopup, RunCustomAction},
	},
	prelude::*,
	routing::Page,
	state::{BackEvent, ModalType},
	util::PtrEq,
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
			.flex()
			.background(theme.footer)
			.border(border_top(theme.border, theme.panel_border).alignment(BorderAlignment::Outer))
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
		let launch_instance = use_mutation(LaunchInstance::new(back_state.clone()));
		let kill_instance = use_mutation(KillInstance::new(back_state.clone()));
		let more_buttons = use_query(Query::new(
			DropdownButtonLocation::TemplateMoreOptions,
			FetchDropdownButtons::new(back_state.clone()),
		));
		let custom_action_mutation = use_mutation(Mutation::new(
			RunCustomAction::new(back_state.clone()).toast(
				&back_state,
				None,
				"Failed to run action",
			),
		));
		let open_popup_mutation = use_mutation(Mutation::new(OpenCustomPopup::new(
			back_state.clone(),
			front_state.clone(),
		)));

		let id = if let FooterItem::InstanceOrTemplate(info) = &self.item {
			Some(info.id.clone())
		} else {
			None
		};
		let instance_run_state = use_query(Query::new(
			id.unwrap_or_default(),
			FetchInstanceRunState::new(back_state.clone()),
		));

		let mut show_install_modal = use_state(|| false);

		let front_state2 = front_state.clone();
		use_future(move || {
			let mut event_rx = front_state2.read().subscribe_events();
			async move {
				while let Ok(ev) = event_rx.recv().await {
					if let BackEvent::UpdateRunningInstances = ev {
						invalidate_all::<FetchInstanceRunState>().await;
					}
				}
			}
		});

		let instance_run_state = instance_run_state
			.read()
			.state()
			.ok()
			.cloned()
			.unwrap_or_default();

		let front_state2 = front_state.clone();
		let left_button = if let FooterItem::InstanceOrTemplate(info) = &self.item {
			match info.ty {
				ConfigKind::Instance => {
					let id = info.id.clone();
					Some(
						icon_button("properties", &theme)
							.on_press(move |_| {
								front_state2.write().navigate(Page::Instance(id.clone()));
							})
							.into_element(),
					)
				}
				ConfigKind::Template => {
					let id = info.id.clone();
					let more_buttons = more_buttons.read();
					let more_buttons = more_buttons.state();
					let more_buttons = more_buttons.ok().cloned().unwrap_or_default();
					let more_buttons2 = more_buttons.clone();

					Some(
						Dropdown::new(
							Selected::Single(TemplateMoreOption::More),
							Rc::new(move |selected| match selected.single() {
								TemplateMoreOption::More => {}
								TemplateMoreOption::Delete => {
									front_state2
										.write()
										.set_modal(Some(ModalType::DeleteTemplate(id.clone())));
								}
								TemplateMoreOption::Custom(idx) => {
									if let Some(button) = more_buttons2.get(idx) {
										run_dropdown_button(
											button,
											Some(id.clone()),
											&custom_action_mutation,
											&open_popup_mutation,
										);
									}
								}
							}),
						)
						.custom_header(SelectOption::new(
							TemplateMoreOption::More,
							"",
							Some("elipsis"),
						))
						.header_width(Size::auto())
						.hide_arrow()
						.options_width(180.0)
						.align_options_top()
						.custom_buttons(more_buttons, TemplateMoreOption::Custom)
						.maybe_child(info.is_deletable, || {
							SelectOption::new(TemplateMoreOption::Delete, "Delete", Some("trash"))
								.tip("Delete this template")
						})
						.into_element(),
					)
				}
				ConfigKind::BaseTemplate => None,
			}
		} else {
			None
		};

		let left = rect()
			.height(Size::fill())
			.width(Size::flex(1.0))
			.cont()
			.main_align(Alignment::End)
			.cross_align(Alignment::Center)
			.maybe_child(left_button);

		let is_non_editable_template = matches!(
			&self.item,
			FooterItem::InstanceOrTemplate(InstanceItemInfo {
				ty: ConfigKind::Template,
				is_editable: false,
				..
			})
		);

		let (fg, border, bg) = if self.item == FooterItem::None || is_non_editable_template {
			(theme.disabled, theme.disabled, theme.bg)
		} else {
			(theme.primary, theme.primary, theme.primary_bg)
		};

		let item = self.item.clone();
		let mut show_install_modal2 = show_install_modal;
		let front_state2 = front_state.clone();
		let on_press = move |_| match &item {
			FooterItem::None => {}
			FooterItem::InstanceOrTemplate(info) => match info.ty {
				ConfigKind::Instance => match instance_run_state {
					InstanceRunState::Stopped => {
						launch_instance.mutate(LaunchInstanceParams {
							id: info.id.clone(),
							offline: false,
						});
					}
					InstanceRunState::Running => {
						kill_instance.mutate((info.id.clone(), None));
					}
				},
				ConfigKind::Template | ConfigKind::BaseTemplate => {
					if is_non_editable_template {
						return;
					}
					front_state2
						.write()
						.set_modal(Some(ModalType::Configuration(info.get_config_item())));
				}
			},
			FooterItem::InstallPackage(..) => {
				show_install_modal2.set(true);
			}
		};

		let center = rect()
			.height(Size::fill())
			.width(Size::px(128.0))
			.center()
			.maybe(is_non_editable_template, |this| {
				this.tip(
					&front_state,
					"This template is from a plugin and cannot be edited",
				)
			})
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
							.child(icon(self.item.icon(instance_run_state), 16.0))
							.child(self.item.title(instance_run_state)),
					),
			);

		let right = rect().height(Size::fill()).width(Size::flex(1.0));

		rect()
			.width(Size::fill())
			.height(Size::px(theme.footer_height))
			.cont()
			.background(theme.footer)
			.child(left)
			.child(center)
			.child(right)
			.maybe(*show_install_modal.read(), |this| {
				this.child(InstallModalHandler {
					req: match &self.item {
						FooterItem::InstallPackage(req) => req.clone(),
						_ => unreachable!(),
					},
					on_close: EventHandler::new(move |_| show_install_modal.set(false)),
				})
			})
	}
}

#[derive(PartialEq, Clone)]
enum TemplateMoreOption {
	More,
	Delete,
	Custom(usize),
}

#[derive(PartialEq)]
struct InstallModalHandler {
	req: ArcPkgReq,
	on_close: EventHandler<()>,
}

impl Component for InstallModalHandler {
	fn render(&self) -> impl IntoElement {
		let back_state = use_consume::<BackState>();
		let details_query = use_query(Query::new(
			self.req.clone(),
			FetchPackageDetails::new(back_state.clone()).toast(
				&back_state,
				None,
				"Failed to fetch package",
			),
		));

		PackageInstallModal {
			req: self.req.clone(),
			meta: PtrEq(
				details_query
					.read()
					.state()
					.ok()
					.map(|x| x.meta.clone())
					.unwrap_or_default(),
			),
			props: PtrEq(
				details_query
					.read()
					.state()
					.ok()
					.map(|x| x.props.clone())
					.unwrap_or_default(),
			),
			on_close: self.on_close.clone(),
		}
	}
}

/// What the footer has selected
#[derive(Clone, PartialEq)]
pub enum FooterItem {
	None,
	InstanceOrTemplate(InstanceItemInfo),
	InstallPackage(ArcPkgReq),
}

impl FooterItem {
	fn icon(&self, run_state: InstanceRunState) -> &'static str {
		match self {
			Self::None => "box",
			Self::InstanceOrTemplate(InstanceItemInfo {
				ty: ConfigKind::Instance,
				..
			}) => match run_state {
				InstanceRunState::Stopped => "play",
				InstanceRunState::Running => "stop",
			},
			Self::InstanceOrTemplate(InstanceItemInfo {
				ty: ConfigKind::Template | ConfigKind::BaseTemplate,
				..
			}) => "properties",
			Self::InstallPackage(..) => "download",
		}
	}

	fn title(&self, run_state: InstanceRunState) -> &'static str {
		match self {
			Self::None => "Select...",
			Self::InstanceOrTemplate(InstanceItemInfo {
				ty: ConfigKind::Instance,
				..
			}) => match run_state {
				InstanceRunState::Stopped => "Launch",
				InstanceRunState::Running => "Kill",
			},
			Self::InstanceOrTemplate(InstanceItemInfo {
				ty: ConfigKind::Template | ConfigKind::BaseTemplate,
				..
			}) => "Edit",
			Self::InstallPackage(..) => "Install",
		}
	}
}
