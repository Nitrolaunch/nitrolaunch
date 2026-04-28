use crate::components::nav::router::Page;

#[derive(Clone)]
pub enum AppEvent {
	RouteChanged(Page),
}
