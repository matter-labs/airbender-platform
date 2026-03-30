use crate::error::{CliError, Result};
use crate::ui;
use airbender_build::clean_reproducible_volumes;

pub fn run() -> Result<()> {
    let count = clean_reproducible_volumes()
        .map_err(|e| CliError::with_source("failed to clean Docker resources", e))?;

    if count == 0 {
        ui::info("no reproducible-build resources found");
    } else {
        ui::success(format!(
            "removed {count} Docker resource{}",
            if count == 1 { "" } else { "s" }
        ));
    }
    Ok(())
}
