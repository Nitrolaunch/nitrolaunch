use crate::prelude::*;

#[derive(PartialEq)]
pub struct ServersPage;

impl Component for ServersPage {
	fn render(&self) -> impl IntoElement {
		rect().expanded()
	}
}
