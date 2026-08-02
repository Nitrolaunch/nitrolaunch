use std::{rc::Rc, time::Duration};

use crate::{
	components::input::select::Selected,
	ops::account::FetchAccounts,
	pages::settings,
	prelude::*,
	state::{ModalType, use_launcher_data},
	util::assets::DEFAULT_SKIN,
};

pub mod auth;

#[derive(PartialEq)]
pub struct AccountSelector;

impl Component for AccountSelector {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();
		let accounts_query = use_query(Query::new((), FetchAccounts::new(back_state.clone())));
		let data = use_launcher_data();

		let default = Vec::new();
		let accounts = accounts_query.read();
		let accounts = accounts.state();
		let accounts = accounts.ok().unwrap_or(&default);

		let data2 = data.clone();
		Dropdown::new(
			Selected::Single(data.data.read().current_account.clone()),
			Rc::new(move |selected| {
				data2.data.clone().write().current_account = selected.single();
				data2.save();
			}),
		)
		.header_width(Size::px(240.0))
		.child(SelectOption::new(None, "No Account", Some("user")))
		.children(accounts.iter().map(|account| {
			let name = account.get_name().unwrap_or(&*account.get_id());

			let image = ImageViewer::new(get_account_image(account.get_uuid()))
				.asset_age(Duration::from_hours(1))
				.width(Size::px(16.0))
				.height(Size::px(16.0))
				.corner_radius(3.0)
				.sampling_mode(SamplingMode::Nearest);

			let front_state = front_state.clone();
			let action_button =
				icon_button("gear", &theme).on_press(move |e: Event<PressEventData>| {
					e.stop_propagation();
					front_state
						.write()
						.set_modal(Some(ModalType::Settings(settings::Tab::Accounts)));
				});

			SelectOption::new_custom_icon(
				Some(account.get_id().to_string()),
				name,
				image.into_element(),
			)
			.action_button(action_button.into_element())
		}))
	}
}

pub fn get_account_image(uuid: Option<&str>) -> ImageSource {
	if let Some(uuid) = uuid {
		Url::parse(&format!("https://www.minepic.org/avatar/{uuid}?overlay"))
			.unwrap_or(Url::parse("https://example.com").unwrap())
			.into()
	} else {
		("default-SKIN", DEFAULT_SKIN).into()
	}
}
