use crate::pages::home::{HomePage, SelectedLocation};
use crate::prelude::*;
use nitrolaunch::config_crate::ConfigKind;
use nitrolaunch::core::util::versions::MinecraftVersion;
use nitrolaunch::shared::Side;
use nitrolaunch::shared::loaders::Loader;

pub struct InstanceListItem {
	info: InstanceItemInfo,
	is_selected: bool,
	list: Entity<HomePage>,
}

impl InstanceListItem {
	pub fn new(
		info: InstanceItemInfo,
		is_selected: bool,
		list: Entity<HomePage>,
		_: &Window,
		_: &mut Context<Self>,
	) -> Self {
		Self {
			info,
			is_selected,
			list,
		}
	}
}

impl Render for InstanceListItem {
	fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
		let name = if let Some(name) = &self.info.name {
			name.clone()
		} else {
			self.info.id.clone()
		};

		let side = show_multi(self.info.side.is_some(), || {
			rsx! {
				<>
					<div>{Icon::empty().path("icons/server.svg")}</div>
					<div>{self.info.side.as_ref().unwrap().to_string_pretty()}</div>
				</>
			}
		});

		let version = show_multi(self.info.version.is_some(), || {
			rsx! {
				<>
					<div>{Icon::empty().path("icons/tag.svg")}</div>
					<div>{self.info.version.as_ref().unwrap().to_string()}</div>
				</>
			}
		});

		let loader = show_multi(self.info.loader.is_some(), || {
			rsx! {
				<>
					<div>{Icon::empty().path("icons/box.svg")}</div>
					<div>{self.info.loader.as_ref().unwrap().to_string()}</div>
				</>
			}
		});

		let id = ElementId::Name(SharedString::new(format!("{}", self.info.id)));
		let list_entity = self.list.clone();
		let location = SelectedLocation {
			id: self.info.id.clone(),
			ty: self.info.ty,
		};

		let colors = if self.is_selected {
			match self.info.ty {
				ConfigKind::Instance => (
					rgb(0x7ee91b).into(),
					rgb(0x7ee91b).into(),
					rgb(0x051d1d).into(),
				),
				ConfigKind::Template | ConfigKind::BaseTemplate => (
					rgb(0x1be9ce).into(),
					rgb(0x1be9ce).into(),
					rgb(0x0d1624).into(),
				),
			}
		} else {
			(
				cx.theme().foreground,
				cx.theme().border,
				cx.theme().secondary,
			)
		};

		let bottom_color = if self.is_selected {
			colors.0
		} else {
			cx.theme().muted_foreground
		};

		rsx! {
			<v_flex id={id} w_full h_24 text_color={colors.0} bg={colors.2} round bordered={cx} border_color={colors.1} cursor_pointer tip={&self.info.id} on_click={move |_, _, cx| {
				let location = location.clone();
				println!("Click");
				list_entity.update(cx, move |this, cx| {
					this.select(location, cx);
				});
			}}>
				<sect h_16 flex flex_row id="header" border_b_2 border_color={colors.1}>
					<center w_16 p_2 id="icon-cont">
						{img("assets/default.png").size(px(12.0))}
					</center>
					<sect start grow=1.0 p_2 id="name-cont" font_bold truncate>{name}</sect>
				</sect>
				<sect grow=0.5 id="under" grid grid_cols=3 text_sm text_color={bottom_color}>
					<cont truncate>{...side}</cont>
					<cont truncate>{...version}</cont>
					<cont truncate>{...loader}</cont>
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
