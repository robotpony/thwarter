//! `thwart package`, `thwart install` and `thwart installed`
//! (REQ-CLI-12 to 16).

use crate::packaging::{install, installed, PackagePlan};
use crate::render::clean_text;
use anyhow::{anyhow, bail, Result};
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

fn data_dir() -> Result<PathBuf> {
    crate::settings::data_dir().ok_or_else(|| anyhow!("neither XDG_DATA_HOME nor HOME is set, so there's nowhere to install worlds"))
}

pub fn package_command(world: String, output: Option<String>, strip_comments: bool, yes: bool) -> Result<()> {
    let plan = PackagePlan::new(Path::new(&world), output.map(PathBuf::from))?;
    // Say what ships before anything is written (REQ-CLI-14).
    eprintln!("{}", clean_text(&plan.notice(strip_comments)));
    if !yes {
        if !std::io::stdin().is_terminal() {
            bail!("not packaging without a yes: run it on a terminal to answer, or pass --yes");
        }
        eprint!("Package and share these details? [y/N] ");
        std::io::stderr().flush()?;
        let mut answer = String::new();
        std::io::stdin().lock().read_line(&mut answer)?;
        if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
            bail!("not packaged");
        }
    }
    let path = plan.write(strip_comments)?;
    println!("Packaged {} files into {}", plan.files.len(), path.display());
    Ok(())
}

pub fn install_command(package: String, replace: bool) -> Result<()> {
    let report = install(Path::new(&package), &data_dir()?, replace)?;
    let m = &report.manifest;
    let what = m.title.clone().unwrap_or_else(|| m.world.clone());
    let version = m.version.as_ref().map(|v| format!(" {}", v)).unwrap_or_default();
    println!(
        "{} {}{} into {}",
        if report.replaced { "Replaced" } else { "Installed" },
        clean_text(&what),
        clean_text(&version),
        report.path.display()
    );
    if let Some(note) = report.persona_note {
        println!("{}", clean_text(&note));
    }
    println!("Play it with: thwart play {}/{}", m.persona, m.world);
    Ok(())
}

pub fn installed_command() -> Result<()> {
    let worlds = installed(&data_dir()?);
    if worlds.is_empty() {
        println!("No worlds installed.");
        return Ok(());
    }
    for w in worlds {
        let title = w.title.map(|t| format!("  {}", t)).unwrap_or_default();
        let version = w.version.map(|v| format!(" ({})", v)).unwrap_or_default();
        println!("{}", clean_text(&format!("{}/{}{}{}", w.persona, w.world, title, version)));
    }
    Ok(())
}
