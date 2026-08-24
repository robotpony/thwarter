use crate::cli::{OutputFormat, OutputFormatter};
use anyhow::Result;

/// Not implemented yet. `WorldEngine` can now load and validate a world
/// (see `validate_command`, `show_command`), but ASCII map rendering
/// (`design/CLI_DESIGN.md`) has not been built on top of it. This command
/// intentionally reports that rather than printing placeholder output, so
/// `thwart map` failing to do anything real is visible instead of hidden.
pub fn map_command(
    _location: Option<String>,
    _player: Option<String>,
    _depth: Option<usize>,
    _style: String,
    _output: Option<String>,
    _connections: bool,
) -> Result<()> {
    let formatter = OutputFormatter::new(OutputFormat::Text);
    formatter.print_warning("thwart map is not implemented yet (tracked in design/CLI_DESIGN.md)");
    Err(anyhow::anyhow!("map command not implemented"))
}
