use anyhow::anyhow;

/// Launches an instance in the background
pub fn launch_instance(instance: &str, account: Option<&str>) -> anyhow::Result<()> {
	super::interface::launch_instance(instance, account).map_err(|e| anyhow!(e))
}
