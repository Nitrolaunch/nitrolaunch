use nitrolaunch::{
	pkg_crate::{metadata::PackageMetadata, properties::PackageProperties},
	shared::{
		pkg::{ArcPkgReq, PackageKind},
		versions::VersionPattern,
	},
};

use crate::{
	components::{
		dialog::modal::{MODAL_MEDIUM_HEIGHT, MODAL_MEDIUM_WIDTH, Modal, ModalButton},
		input::{tabs::TopTabs, text::TextInput},
	},
	ops::{
		instance::{FetchItems, InstanceItemInfo, InstancesAndTemplates},
		packages::{InstallPackage, PackageInstallLocation},
	},
	prelude::*,
	util::{PtrEq, assets::get_instance_icon},
};

#[derive(PartialEq)]
pub struct PackageInstallModal {
	pub req: ArcPkgReq,
	pub meta: PtrEq<PackageMetadata>,
	pub props: PtrEq<PackageProperties>,
	pub on_close: EventHandler<()>,
}

impl Component for PackageInstallModal {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let install_package = use_mutation(Mutation::new(
			InstallPackage::new(back_state.clone()).toast(
				&back_state,
				Some("Package installed"),
				"Failed to install package",
			),
		));
		let items_query = use_query(FetchItems::new(back_state.clone()));
		let tab = use_state(|| Tab::Instance);
		let selected_item = use_state::<Option<String>>(|| None);
		let new_instance_id = use_state(|| String::new());

		let tab2 = tab.clone();
		let mut selected_item2 = selected_item.clone();
		use_side_effect(move || {
			tab2.read();
			selected_item2.set(None);
		});

		let name = self.meta.0.name.clone().unwrap_or(self.req.to_string());
		let is_modpack = self.props.0.kinds.contains(&PackageKind::Modpack);

		let tabs = TopTabs::new(tab).children(
			Tab::get_tabs(is_modpack)
				.into_iter()
				.map(|x| SelectOption::new(x.clone(), x.name(is_modpack), Some(x.icon()))),
		);

		let version_indicator = if let VersionPattern::Single(version) = &self.req.content_version {
			format!("Installing version {version}")
		} else {
			"Installing best version".into()
		};
		let version_indicator = rect()
			.width(Size::fill())
			.height(Size::px(48.0))
			.padding(theme.gap2 * 2.0)
			.horizontal()
			.spacing(theme.gap2)
			.cross_align(Alignment::Center)
			.color(theme.fg2)
			.border(border_bottom(theme.border, theme.panel_border))
			.child(icon("tag", 12.0))
			.child(version_indicator);

		let default_items = InstancesAndTemplates::default();
		let items = items_query.read();
		let items = items.state();
		let items = items.ok().unwrap_or(&default_items);

		let tab_contents = match &*tab.read() {
			Tab::Instance => {
				let out = grid(
					3,
					items.instances.iter().map(|x| Item {
						item: x.clone(),
						selected: selected_item.clone(),
					}),
				)
				.gap(theme.gap2);

				ScrollView::new().expanded().child(out).into_element()
			}
			Tab::Template => {
				let out = grid(
					3,
					items.templates.iter().map(|x| Item {
						item: x.clone(),
						selected: selected_item.clone(),
					}),
				)
				.gap(theme.gap2);

				ScrollView::new().expanded().child(out).into_element()
			}
			Tab::BaseTemplate => rect()
				.center()
				.child(placeholder("Package will be installed globally", &theme))
				.into_element(),
			Tab::ModpackInstance => rect()
				.width(Size::fill())
                .padding(theme.gap2)
				.child(field(
					"ID for new instance",
					"hashtag",
					&theme,
					TextInput::new(new_instance_id),
				))
				.into_element(),
		};

		let contents = rect().flex().child(tabs).child(version_indicator).child(
			rect()
				.width(Size::fill())
				.height(Size::flex(1.0))
				.child(tab_contents),
		);

		let is_save_ready = match &*tab.read() {
			Tab::BaseTemplate => true,
			_ => selected_item.read().is_some(),
		};

		let req = self.req.clone();
		let tab = tab.clone();
		let selected_item = selected_item.clone();
		let on_close = self.on_close.clone();
		Modal::new(format!("Install {name}"), "download".into())
			.size(MODAL_MEDIUM_WIDTH, MODAL_MEDIUM_HEIGHT)
			.maybe_child(true, || contents)
			.on_close(self.on_close.clone())
			.cancel_button()
			.button(ModalButton {
				title: "Install".into(),
				icon: "download".into(),
				on_click: (move |_| {
					let location = match &*tab.read() {
						Tab::Instance => {
							let Some(selected) = selected_item.read().clone() else {
								return;
							};
							if is_modpack {
								PackageInstallLocation::InstanceModpack(selected.into())
							} else {
								PackageInstallLocation::Instance(selected.into())
							}
						}
						Tab::Template => {
							let Some(selected) = selected_item.read().clone() else {
								return;
							};
							if is_modpack {
								PackageInstallLocation::TemplateModpack(selected.into())
							} else {
								PackageInstallLocation::Template(selected.into(), None)
							}
						}
						Tab::BaseTemplate => PackageInstallLocation::BaseTemplate(None),
						Tab::ModpackInstance => {
							let Some(selected) = selected_item.read().clone() else {
								return;
							};
							PackageInstallLocation::NewInstanceModpack(selected.into())
						}
					};

					install_package.mutate((req.clone(), location));
					on_close.call(());
				})
				.into(),
				active: is_save_ready,
			})
	}
}

#[derive(PartialEq)]
struct Item {
	item: InstanceItemInfo,
	selected: State<Option<String>>,
}

impl Component for Item {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);

		let is_selected = self.selected.read().as_ref() == Some(&self.item.id);

		let mut selected = self.selected.clone();
		let id = self.item.id.clone();

		let inst_icon = if self.item.icon.is_none() {
			icon("box", 28.0).into_element()
		} else {
			let inst_icon = get_instance_icon(self.item.icon.as_deref());
			ImageViewer::new(inst_icon)
				.width(Size::px(28.0))
				.height(Size::px(28.0))
				.into_element()
		};
		let inst_icon = rect()
			.width(Size::px(48.0))
			.height(Size::px(48.0))
			.center()
			.child(inst_icon);

		rect()
			.width(Size::fill())
			.height(Size::px(48.0))
			.panel_colorway(&theme, *is_hovered.read(), is_selected)
			.hover(is_hovered)
			.corner_radius(theme.round)
			.on_press(move |_| {
				selected.set(Some(id.clone()));
			})
			.clickable()
			.cont()
			.child(inst_icon)
			.child(
				segment(self.item.name(), 1.0)
					.height(Size::fill())
					.main_align(Alignment::Center),
			)
	}
}

#[derive(PartialEq, Clone)]
enum Tab {
	Instance,
	Template,
	BaseTemplate,
	ModpackInstance,
}

impl Tab {
	fn get_tabs(is_modpack: bool) -> &'static [Tab] {
		if is_modpack {
			&[Self::Instance, Self::Template, Self::ModpackInstance]
		} else {
			&[Self::Instance, Self::Template, Self::BaseTemplate]
		}
	}

	fn name(&self, is_modpack: bool) -> &str {
		match self {
			Self::Instance if is_modpack => "Existing Instance",
			Self::Instance => "Instance",
			Self::Template if is_modpack => "Existing Template",
			Self::Template => "Template",
			Self::BaseTemplate => "Base Template",
			Self::ModpackInstance => "New Instance",
		}
	}

	fn icon(&self) -> &'static str {
		match self {
			Self::Instance => "box",
			Self::Template => "diagram",
			Self::BaseTemplate => "diagram",
			Self::ModpackInstance => "minecraft",
		}
	}
}
