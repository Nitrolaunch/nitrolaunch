use gpui::prelude::*;
use gpui::*;
use nitrolaunch::config_crate::ConfigKind;

use crate::components::instance::{InstanceItemInfo, InstanceListItem};
use crate::state::AppState;
use crate::util::state::{Resource, ResourceState, Trigger};

pub struct HomePage {
	app_state: AppState,
	instances: Resource<Vec<InstanceItemInfo>>,
	visible_trigger: Trigger,
}

impl HomePage {
	pub fn new(app_state: AppState, _: &Window, _: &mut Context<Self>) -> Self {
		Self {
			app_state,
			instances: Resource::new(),
			visible_trigger: Trigger::new(),
		}
	}

	fn fetch_instances(&self, cx: &mut Context<Self>) {
		let app_state = self.app_state.clone();

		self.instances.fetch(cx, async move |_| {
			// return Ok(Vec::new());
			println!("Start config");
			let config = app_state.config().await?;
			println!("Finish config");
			
			let instances = config.instances.values().map(|x| InstanceItemInfo {
				id: x.id().to_string(),
				ty: ConfigKind::Instance,
			});
			println!("Finish");

			Ok(instances.collect())
		});
	}

	pub fn visible(&mut self) {
		self.visible_trigger.trigger();
	}
}

impl Render for HomePage {
	fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
		if self.visible_trigger.check() {
			self.fetch_instances(cx);
		}

		let instances = match self.instances.state().as_deref() {
			None | Some(ResourceState::Loading | ResourceState::Err(..)) => {
				vec![div().into_any_element()]
			}
			Some(ResourceState::Loaded(instances)) => instances
				.iter()
				.map(|x| {
					cx.new(|cx| InstanceListItem::new(x.id.clone(), x.ty.clone(), window, cx))
						.into_any_element()
				})
				.collect(),
		};

		gpui_rsx::rsx! {
			<div size_full flex grid grid_cols=5>{...instances}</div>
		}
	}
}
