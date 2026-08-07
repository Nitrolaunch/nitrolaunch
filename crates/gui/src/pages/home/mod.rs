use std::rc::Rc;

use crate::{
	components::{
		footer::FooterItem,
		input::{
			select::{Selected, run_dropdown_button},
			tabs::TopTabs,
			text::{TextInput, search_bar},
		},
		instance::transfer::InstanceTransferMode,
	},
	ops::{
		instance::{FetchItems, InstanceItemInfo, InstancesAndTemplates},
		plugin_results::{FetchDropdownButtons, OpenCustomPopup, RunCustomAction},
	},
	pages::{config::ConfiguredItem, home::item::InstanceListItem},
	prelude::*,
	state::ModalType,
};
use nitrolaunch::{
	config_crate::ConfigKind, plugin_crate::hook::hooks::DropdownButtonLocation, shared::Side,
};

pub mod item;

#[derive(PartialEq)]
pub struct HomePage;

impl Component for HomePage {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let front_state = use_front_state();
		let items_query = use_query(FetchItems::new(back_state.clone()));
		let add_buttons = use_query(Query::new(
			DropdownButtonLocation::AddTemplateOrInstance,
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

		let tab = use_state(|| Tab::Instances);
		let filter = use_state::<Option<Side>>(|| None);
		let search = use_state(String::new);
		let selected = use_state::<Option<InstanceItemInfo>>(|| None);

		let front_state2 = front_state.clone();
		use_side_effect(move || {
			if let Some(selected) = selected.read().clone() {
				front_state2
					.write()
					.set_footer(FooterItem::InstanceOrTemplate(selected));
			}
		});

		let items_gap = 20.0;
		let items_side_padding = 0.0;
		let items = items_query.read();
		let items = match &*items.state() {
			QueryStateData::Pending
			| QueryStateData::Loading { res: _ }
			| QueryStateData::Settled { res: Err(..), .. } => InstancesAndTemplates {
				instances: Vec::new(),
				templates: Vec::new(),
			},
			QueryStateData::Settled { res: Ok(res), .. } => res.clone(),
		};

		let items = match &*tab.read() {
			Tab::Instances => &items.instances,
			Tab::Templates => &items.templates,
		};

		let add_placeholder_ty = match &*tab.read() {
			Tab::Instances => ConfigKind::Instance,
			Tab::Templates => ConfigKind::Template,
		};
		let add_placeholder =
			InstanceListItem::add_placeholder(add_placeholder_ty, selected);

		let items = items
			.iter()
			.filter(|x| {
				if let Some(filter) = &*filter.read()
					&& x.side != Some(*filter)
				{
					return false;
				}

				if !search.read().is_empty() {
					let search = search.read().to_lowercase();
					let name = x.name.as_deref().unwrap_or_default().to_lowercase();
					let id = x.id.to_lowercase();
					if !name.contains(&search) && !id.contains(&search) {
						return false;
					}
				}

				true
			})
			.map(|x| InstanceListItem::new(x.clone(), selected))
			.chain(std::iter::once(add_placeholder));

		let items_elem = grid(4, items).gap(items_gap);

		let items_elem = rect().child(items_elem).width(Size::fill());

		let add_buttons = add_buttons.read();
		let add_buttons = add_buttons.state();
		let add_buttons = add_buttons.ok().cloned().unwrap_or_default();
		let add_buttons2 = add_buttons.clone();
		let front_state2 = front_state.clone();
		let add_dropdown = Dropdown::new(
			Selected::Single(AddOption::Add),
			Rc::new(move |selected| match selected.single() {
				AddOption::Add => {}
				AddOption::Instance => {
					front_state2
						.write()
						.set_modal(Some(ModalType::Configuration(ConfiguredItem {
							id: None,
							ty: ConfigKind::Instance,
							is_new: true,
						})))
				}
				AddOption::Template => {
					front_state2
						.write()
						.set_modal(Some(ModalType::Configuration(ConfiguredItem {
							id: None,
							ty: ConfigKind::Template,
							is_new: true,
						})))
				}
				AddOption::ImportInstance => front_state2.write().set_modal(Some(
					ModalType::Transfer(InstanceTransferMode::Import, None),
				)),
				AddOption::MigrateInstances => {
					front_state2.write().set_modal(Some(ModalType::Migrate))
				}
				AddOption::Custom(id) => {
					if let Some(button) = add_buttons2.get(id) {
						run_dropdown_button(
							button,
							None,
							&custom_action_mutation,
							&open_popup_mutation,
						);
					}
				}
			}),
		)
		.custom_header(SelectOption::new(AddOption::Add, "Add", Some("plus")))
		.header_width(Size::px(80.0))
		.options_width(180.0)
		.hide_arrow()
		.panel_colorway()
		.child(SelectOption::new(
			AddOption::Instance,
			"New Instance",
			Some("box"),
		))
		.child(SelectOption::new(
			AddOption::Template,
			"New Template",
			Some("diagram"),
		))
		.child(SelectOption::new(
			AddOption::ImportInstance,
			"Import Instance",
			Some("download"),
		))
		.child(SelectOption::new(
			AddOption::MigrateInstances,
			"Migrate Instances",
			Some("shuffle"),
		))
		.custom_buttons(add_buttons, AddOption::Custom);

		let tabs = TopTabs::new(tab)
			.child(SelectOption::new(Tab::Instances, "Instances", Some("box")))
			.child(SelectOption::new(
				Tab::Templates,
				"Templates",
				Some("diagram"),
			));

		let bar_left = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.cont()
			.cross_align(Alignment::Center)
			.child(add_dropdown)
			.child(segment(tabs, 1.0));

		let search_bar = search_bar(TextInput::new(search), &theme);

		let bar_center = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.center()
			.child(search_bar);

		let filters = TopTabs::new(filter)
			.child(SelectOption::new(None, "All", Some("box")))
			.child(SelectOption::new(
				Some(Side::Client),
				"Client",
				Some("controller"),
			))
			.child(SelectOption::new(
				Some(Side::Server),
				"Server",
				Some("server"),
			));

		let bar_right = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.center()
			.child(filters);

		let bar_elem = rect()
			.width(Size::fill())
			.height(Size::px(32.0))
			.cont()
			.padding((3.0, items_gap))
			.child(bar_left)
			.child(bar_center)
			.child(bar_right);

		let view = rect().flex().child(bar_elem).child(items_elem);

		let view = ScrollView::new().expanded().child(view);

		rect().fill().child(view).padding(Gaps::new(
			theme.gap2,
			items_side_padding,
			0.0,
			items_side_padding,
		))
	}
}

#[derive(PartialEq, Clone)]
enum Tab {
	Instances,
	Templates,
}

#[derive(PartialEq, Clone)]
enum AddOption {
	Add,
	Instance,
	Template,
	ImportInstance,
	MigrateInstances,
	Custom(usize),
}
