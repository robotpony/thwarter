use crate::cli::{OutputFormat, OutputFormatter};
use anyhow::Result;

/// Not implemented yet. See the note in `map.rs`: this reports its own
/// absence instead of returning fabricated results.
pub fn search_command(
    _query: String,
    _location: Option<String>,
    _search_type: Option<String>,
    _case_sensitive: bool,
    _regex: bool,
    _missing_connections: bool,
) -> Result<()> {
    let formatter = OutputFormatter::new(OutputFormat::Text);
    formatter.print_warning("thwart search is not implemented yet (tracked in design/CLI_DESIGN.md)");
    Err(anyhow::anyhow!("search command not implemented"))
}
