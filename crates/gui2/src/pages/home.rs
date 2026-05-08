use std::rc::Rc;

use crate::{
	components::{footer::FooterItem, instance::InstanceListItem},
	ops::instance::{FetchItems, InstanceItemInfo, InstancesAndTemplates},
	prelude::*,
};
use nitrolaunch::shared::Side;

#[derive(PartialEq)]
pub struct HomePage;

impl Component for HomePage {
	fn render(&self) -> impl IntoElement {
		let back_state = use_consume::<BackState>();
		let front_state = use_front_state();
		let items_query = use_query(FetchItems::new(back_state.clone()));

		let tab = use_state(|| "instances".to_string());
		let filter = use_state(|| "all".to_string());
		let selected = use_state::<Option<InstanceItemInfo>>(|| None);

		use_side_effect(move || {
			if let Some(selected) = selected.read().clone() {
				front_state
					.write()
					.set_footer(FooterItem::InstanceOrTemplate(selected));
			}
		});

		let items_gap = 20.0;
		let items_side_padding = 24.0;
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

		let items = match tab.read().as_str() {
			"instances" => &items.instances,
			"templates" => &items.templates,
			_ => unreachable!(),
		};

		let items = items
			.into_iter()
			.filter(|x| {
				if &*filter.read() == "client" && x.side != Some(Side::Client) {
					false
				} else if &*filter.read() == "server" && x.side != Some(Side::Server) {
					false
				} else {
					true
				}
			})
			.map(|x| InstanceListItem::new(x.clone(), selected.clone()));

		let items_elem = grid(3, items).gap(items_gap);

		let items_elem = rect().child(items_elem).width(Size::fill());

		let on_select_tab = Rc::new(move |new_tab| tab.clone().set(new_tab));
		let tabs = InlineSelect::new(Some(tab.read().clone()), on_select_tab)
			.child(SelectOption {
				id: "instances".into(),
				title: "Instances".into(),
				icon: Some("box".into()),
			})
			.child(SelectOption {
				id: "templates".into(),
				title: "Templates".into(),
				icon: Some("diagram".into()),
			});

		let bar_left = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.cont()
			.cross_align(Alignment::Center)
			.child(rect().width(Size::px(350.0)).child(tabs));

		let bar_center = rect().width(Size::flex(1.0));

		let on_select_filter = Rc::new(move |new_filter| filter.clone().set(new_filter));
		let filters = InlineSelect::new(Some(filter.read().clone()), on_select_filter)
			.align_end()
			.child(SelectOption {
				id: "all".into(),
				title: "All".into(),
				icon: Some("box".into()),
			})
			.child(SelectOption {
				id: "client".into(),
				title: "Client".into(),
				icon: Some("controller".into()),
			})
			.child(SelectOption {
				id: "server".into(),
				title: "Server".into(),
				icon: Some("server".into()),
			});

		let bar_right = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.cont()
			.cross_align(Alignment::Center)
			.main_align(Alignment::End)
			.child(rect().width(Size::px(350.0)).child(filters));

		let bar_elem = rect()
			.width(Size::fill())
			.height(Size::px(32.0))
			.cont()
			.padding((3.0, items_gap))
			.child(bar_left)
			.child(bar_center)
			.child(bar_right);

		let view = rect().flex().child(bar_elem).child(items_elem);

		let view = ScrollView::new()
			.child(view)
			.width(Size::fill())
			.height(Size::fill());

		rect().fill().child(view).padding((0.0, items_side_padding))
	}
}
