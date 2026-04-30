use crate::prelude::*;
use itertools::Itertools;
use nitrolaunch::config_crate::ConfigKind;

use crate::components::instance::{InstanceItemInfo, InstanceListItem};

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
			let config = app_state.config().await?;

			let instances = config
				.instances
				.values()
				.sorted_by_cached_key(|x| x.id())
				.map(|x| InstanceItemInfo {
					id: x.id().to_string(),
					ty: ConfigKind::Instance,
					name: x.config().name.clone(),
					side: Some(x.side()),
					version: Some(x.version().clone()),
					loader: Some(x.loader().clone()),
				});

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
					cx.new(|cx| InstanceListItem::new(x.clone(), window, cx))
						.into_any_element()
				})
				.collect(),
		};

		rsx! {
			<v_flex id="home-container" size_full overflow_y_scrollbar>
				<div grid grid_cols=5 gap_5 p_3 px_8>{...instances}</div>
			</v_flex>
		}
	}
}
