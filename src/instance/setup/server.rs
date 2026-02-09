use super::super::update::manager::UpdateMethodResult;
use super::{InstKind, Instance};

impl Instance {
	/// Set up data for a server
	pub async fn setup_server(&mut self) -> anyhow::Result<UpdateMethodResult> {
		debug_assert!(matches!(self.kind, InstKind::Server { .. }));

		let out = UpdateMethodResult::new();

		self.ensure_dir()?;

		Ok(out)
	}
}
