use nitrolaunch::config_crate::ConfigKind;

use crate::{
	components::instance::InstanceItemInfo,
	dependency::BackDependency,
	ops::{
		instance::FetchItems,
		launch::{FetchRunningInstances, KillInstance},
	},
	prelude::*,
	state::BackEvent,
	util::assets::get_instance_icon,
};

const ITEM_SIZE: f32 = 28.0;

#[derive(PartialEq)]
pub struct RunningInstances;

impl Component for RunningInstances {
	fn render(&self) -> impl IntoElement {
		let back_state = use_consume::<BackState>();
		let front_state = use_front_state();
		let event_tx = front_state.read().subscribe_events();
		let items_query = use_query(FetchItems::new(back_state.clone()));
		let running_instances = use_query(FetchRunningInstances::new(back_state));

		use_future(move || {
			let mut event_tx = event_tx.resubscribe();
			async move {
				loop {
					if let Ok(BackEvent::UpdateRunningInstances) = event_tx.recv().await {
						BackDependency::RunningInstances.invalidate();
					}
				}
			}
		});

		let items = items_query.read().state().ok().cloned().unwrap_or_default();
		let running_instances = match running_instances.read().state().ok() {
			Some(res) => res
				.iter()
				.map(|x| {
					let item = items
						.instances
						.iter()
						.find(|y| y.id == x.instance_id)
						.cloned()
						.unwrap_or(InstanceItemInfo {
							id: x.instance_id.clone(),
							ty: ConfigKind::Instance,
							name: None,
							icon: None,
							side: None,
							version: None,
							loader: None,
						});

					RunningInstance {
						instance_id: x.instance_id.clone(),
						account: x.account.clone(),
						item,
					}
					.into_element()
				})
				.collect(),
			None => Vec::new(),
		};

		rect()
			.width(Size::fill())
			.height(Size::fill())
			.cont()
			.main_align(Alignment::Start)
			.cross_align(Alignment::Center)
			.padding(10.0)
			.children(running_instances)
	}
}

#[derive(PartialEq)]
struct RunningInstance {
	instance_id: String,
	account: Option<String>,
	item: InstanceItemInfo,
}

impl Component for RunningInstance {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);
		let back_state = use_consume::<BackState>();
		let on_kill = use_mutation(KillInstance::new(
			self.instance_id.clone(),
			self.account.clone(),
			back_state,
		));

		let icon = get_instance_icon(self.item.icon.as_deref());

		let out = rect()
			.center()
			.width(Size::px(ITEM_SIZE))
			.height(Size::px(ITEM_SIZE))
			.item_colorway(&theme, *is_hovered.read(), false)
			.corner_radius(ITEM_SIZE / 2.0)
			.hover(is_hovered)
			.on_press(move |_| on_kill.mutate(()))
			.child(
				ImageViewer::new(icon)
					.width(Size::px(24.0))
					.height(Size::px(24.0)),
			);

		let indicator = if *is_hovered.read() {
			Some(
				rect()
					.padding(6.0)
					.margin((0.0, 0.0, 8.0, 0.0))
					.child(self.instance_id.as_str()),
			)
		} else {
			None
		};

		Attached::new(out).top().maybe_child(indicator)
	}
}
