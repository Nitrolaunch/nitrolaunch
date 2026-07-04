use crate::prelude::*;

#[derive(PartialEq)]
pub struct TabbedBody<T: PartialEq> {
    pub tab: T,
    pub tabs: Vec<T>,
}
