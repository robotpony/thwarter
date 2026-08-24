use crate::cli::{OutputFormat, OutputFormatter};
use anyhow::Result;

/// Not implemented yet. See the note in `map.rs`: this reports its own
/// absence instead of printing invented memory/performance numbers.
/// `--check-references` overlaps with `thwart validate`; use that command
/// in the meantime.
pub fn debug_command(
    _location: Option<String>,
    _check_references: bool,
    _memory_usage: bool,
    _trace_action: Option<String>,
    _profile: bool,
) -> Result<()> {
    let formatter = OutputFormatter::new(OutputFormat::Text);
    formatter.print_warning("thwart debug is not implemented yet (tracked in design/CLI_DESIGN.md)");
    formatter.print_info("for reference checking today, use: thwart validate --strict");
    Err(anyhow::anyhow!("debug command not implemented"))
}
