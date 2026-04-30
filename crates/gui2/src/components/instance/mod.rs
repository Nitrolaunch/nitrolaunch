use crate::prelude::*;
use nitrolaunch::config_crate::ConfigKind;
use nitrolaunch::core::util::versions::MinecraftVersion;
use nitrolaunch::shared::Side;
use nitrolaunch::shared::loaders::Loader;

pub struct InstanceListItem {
	info: InstanceItemInfo,
}

impl InstanceListItem {
	pub fn new(info: InstanceItemInfo, _: &Window, _: &mut Context<Self>) -> Self {
		Self { info }
	}
}

impl Render for InstanceListItem {
	fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
		let name = if let Some(name) = &self.info.name {
			name.clone()
		} else {
			self.info.id.clone()
		};

		rsx! {
			<v_flex w_full h_24 bg={cx.theme().secondary} round bordered={cx} cursor_pointer>
				<sect h_16 flex flex_row id="header" border_b_2 border_color={cx.theme().border}>
					<center w_16 p_2 id="icon-cont">
						{img("assets/default.png").size(px(12.0))}
					</center>
					<sect start grow=1.0 p_2 id="name-cont" font_bold truncate>{name}</sect>
				</sect>
				<sect grow=0.5 id="under" grid grid_cols=3 text_color={cx.theme().muted_foreground}>
					<center truncate><show show={self.info.side.is_some()}>
						<cont>
							<div>{Icon::empty().path("icons/server.svg")}</div>
							<div>{self.info.side.as_ref().unwrap().to_string_pretty()}</div>
						</cont>
					</show></center>
					<center truncate><show show={self.info.version.is_some()}>
						<cont>
							<div>{Icon::empty().path("icons/tag.svg")}</div>
							<div>{self.info.version.as_ref().unwrap().to_string()}</div>
						</cont>
					</show></center>
					<center truncate><show show={self.info.loader.is_some()}>
						<cont>
							<div>{Icon::empty().path("icons/box.svg")}</div>
							<div>{self.info.loader.as_ref().unwrap().to_string()}</div>
						</cont>
					</show></center>
				</sect>
			</v_flex>
		}
	}
}

/// Simple info about an instance or template
#[derive(Clone)]
pub struct InstanceItemInfo {
	pub id: String,
	pub ty: ConfigKind,
	pub name: Option<String>,
	pub side: Option<Side>,
	pub version: Option<MinecraftVersion>,
	pub loader: Option<Loader>,
}
