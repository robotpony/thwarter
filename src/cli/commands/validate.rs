use crate::cli::{AlreadyReported, OutputFormat, OutputFormatter};
use crate::engines::WorldEngine;
use anyhow::Result;

pub fn validate_command(
    world_path: String,
    location: Option<String>,
    strict: bool,
    output: Option<String>,
    format: String,
    fix: bool,
) -> Result<()> {
    let formatter = OutputFormatter::new(OutputFormat::from(format.as_str()));

    if fix {
        formatter.print_warning("--fix is not implemented yet; no files will be modified");
    }
    if let Some(loc) = &location {
        formatter
            .print_warning(&format!("scoping validation to `{}` is not implemented yet; validating the whole world", loc));
    }

    formatter.print_info(&format!("Validating world at {}", world_path));

    let mut engine = WorldEngine::new();
    let report = engine.load_world(&world_path)?;

    formatter.print_info(&format!(
        "Loaded {} location(s), {} character(s), {} object(s)",
        report.locations_loaded,
        report.characters.len(),
        report.objects_loaded
    ));

    let validation = engine.validation(strict);
    for warning in &validation.warnings {
        formatter.print_warning(warning);
    }
    for problem in &validation.problems {
        formatter.print_error(problem);
    }
    if validation.passed() {
        formatter.print_success(&validation.summary());
    } else {
        formatter.print_error(&validation.summary());
    }

    if let Some(output_file) = output {
        let report_text = validation.problems.join("\n");
        std::fs::write(&output_file, report_text)?;
        formatter.print_info(&format!("Validation report saved to: {}", output_file));
    }

    if validation.passed() {
        Ok(())
    } else {
        Err(AlreadyReported.into())
    }
}
