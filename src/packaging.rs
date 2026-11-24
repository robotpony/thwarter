//! Packaging and installing worlds (`design/CLI_REQUIREMENTS.md` ›
//! REQ-CLI-12 to 16; REQ-CFG-5, 7, 8; REQ-WS-6). A package is a zip file
//! named `<persona>-<world>-<version>.thwarter` holding a manifest, the
//! persona's file and the world.

use crate::engines::persona::Persona;
use crate::engines::WorldEngine;
use crate::parsers::response_marker;
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

/// The manifest's name inside a package, and inside an installed world
/// (as a dot file, which the loader skips).
pub const MANIFEST: &str = "thwarter-package.toml";
const INSTALLED_MANIFEST: &str = ".thwarter-package.toml";

/// The most an archive may unpack to (REQ-CFG-8).
pub const MAX_TOTAL_BYTES: u64 = 200 * 1024 * 1024;
pub const MAX_FILES: usize = 10_000;

/// `thwarter-package.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    pub persona: String,
    pub world: String,
    pub title: Option<String>,
    pub version: Option<String>,
    /// The `thwart` version that packaged it.
    pub thwart: String,
    /// When, as seconds since 1970.
    pub packaged: u64,
}

/// What `thwart package` will write, worked out before anything is.
#[derive(Debug, Clone)]
pub struct PackagePlan {
    pub manifest: Manifest,
    /// The persona, if its folder was found; its file ships if it has one.
    pub persona: Option<Persona>,
    /// The world's files, relative to the world folder, sorted.
    pub files: Vec<PathBuf>,
    /// Author comments in the files, and how many files hold them.
    pub comments: usize,
    pub comment_files: usize,
    pub world_root: PathBuf,
    pub output: PathBuf,
}

/// An author comment: `<!-- ... -->` that isn't a response marker.
fn comment_spans(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut spans = Vec::new();
    let mut from = 0;
    while let Some(start) = text[from..].find("<!--").map(|i| i + from) {
        let end = text[start..].find("-->").map(|i| start + i + 3).unwrap_or(text.len());
        if response_marker(&text[start..end]).is_none() {
            spans.push(start..end);
        }
        from = end;
    }
    spans
}

/// `text` without its author comments; response markers stay. A comment
/// alone on its line takes the line with it.
pub fn strip_author_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for span in comment_spans(text) {
        let line_start = text[..span.start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[span.end..].find('\n').map_or(text.len(), |i| span.end + i + 1);
        let alone = text[line_start..span.start].trim().is_empty() && text[span.end..line_end].trim().is_empty();
        let (cut_start, cut_end) = if alone { (line_start.max(last), line_end) } else { (span.start, span.end) };
        out.push_str(&text[last..cut_start]);
        last = cut_end;
    }
    out.push_str(&text[last..]);
    out
}

fn is_safe_segment(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\', ':']) && !name.chars().any(char::is_control)
}

impl PackagePlan {
    /// Plan a package of the world at `world_root`: it must pass
    /// `validate`. Its persona is the one `world/index.md` names, else the
    /// folder the world sits in.
    pub fn new(world_root: &Path, output: Option<PathBuf>) -> Result<Self> {
        let world_root = std::fs::canonicalize(world_root).with_context(|| format!("no world at {}", world_root.display()))?;
        let mut engine = WorldEngine::new();
        engine.load_world(&world_root)?;
        let problems = engine.current_problems(false);
        if !problems.is_empty() {
            bail!("the world has {} problem(s), so it can't be packaged; run `thwart validate` to list them", problems.len());
        }
        let world_id = world_root.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let persona_id = engine
            .world()
            .author
            .clone()
            .or_else(|| world_root.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).map(str::to_string))
            .ok_or_else(|| anyhow!("the world names no author, and its folder has no persona folder around it"))?;
        for (what, id) in [("persona", &persona_id), ("world", &world_id)] {
            if !is_safe_segment(id) {
                bail!("the {} id `{}` can't name a folder in a package", what, id);
            }
        }

        let mut files = Vec::new();
        let mut comments = 0;
        let mut comment_files = 0;
        for entry in walkdir::WalkDir::new(&world_root).follow_links(false).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue; // links and folders aren't packaged
            }
            let rel = entry.path().strip_prefix(&world_root).unwrap().to_path_buf();
            if engine.is_ignored(&rel) || rel.starts_with("world/styles") {
                continue;
            }
            if rel.extension().and_then(|e| e.to_str()) == Some("md") {
                let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
                let n = comment_spans(&text).len();
                comments += n;
                comment_files += usize::from(n > 0);
            }
            files.push(rel);
        }
        files.sort();

        let w = engine.world();
        let manifest = Manifest {
            format: 1,
            persona: persona_id.clone(),
            world: world_id.clone(),
            title: w.title.clone(),
            version: w.version.clone(),
            thwart: env!("CARGO_PKG_VERSION").to_string(),
            packaged: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()),
        };
        let name = match &manifest.version {
            Some(v) if is_safe_segment(v) => format!("{}-{}-{}.thwarter", persona_id, world_id, v),
            _ => format!("{}-{}.thwarter", persona_id, world_id),
        };
        Ok(PackagePlan {
            manifest,
            persona: engine.persona().cloned(),
            files,
            comments,
            comment_files,
            world_root,
            output: output.unwrap_or_else(|| PathBuf::from(name)),
        })
    }

    /// What ships that the author should know about (REQ-WS-6,
    /// REQ-CLI-14): the persona's pen name and contact details, and the
    /// author comments.
    pub fn notice(&self, strip_comments: bool) -> String {
        let mut out = Vec::new();
        match &self.persona {
            Some(p) => {
                let from = p.path.as_ref().map_or("its folder name; it has no index.md".to_string(), |f| f.display().to_string());
                out.push(format!("This package ships as {}, with these details from {}:", p.title, from));
                out.push(format!("  pen name: {}", p.title));
                for (key, value) in &p.contacts {
                    out.push(format!("  {}: {}", key, value));
                }
                if p.contacts.is_empty() {
                    out.push("  (no contact details)".to_string());
                }
            }
            None => out.push(format!("This package ships as `{}`, with no persona file and no contact details.", self.manifest.persona)),
        }
        out.push(if strip_comments {
            format!("Author comments are left out ({} in {} file(s)).", self.comments, self.comment_files)
        } else if self.comments == 0 {
            "It holds no author comments.".to_string()
        } else {
            format!(
                "It holds {} author comment(s) in {} file(s), which anyone who opens it can read; --strip-comments leaves them out.",
                self.comments, self.comment_files
            )
        });
        out.join("\n")
    }

    /// Write the package, atomically: to a temporary file beside the
    /// output, then renamed over it.
    pub fn write(&self, strip_comments: bool) -> Result<PathBuf> {
        let dir = self.output.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let tmp = dir.join(format!(".{}.{}.tmp", self.output.file_name().and_then(|n| n.to_str()).unwrap_or("package"), std::process::id()));
        let result = (|| -> Result<()> {
            let file = std::fs::File::create(&tmp).with_context(|| format!("can't create {}", tmp.display()))?;
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            zip.start_file(MANIFEST, options)?;
            zip.write_all(toml::to_string_pretty(&self.manifest)?.as_bytes())?;
            if let Some(path) = self.persona.as_ref().and_then(|p| p.path.as_ref()) {
                zip.start_file(format!("{}/index.md", self.manifest.persona), options)?;
                zip.write_all(&std::fs::read(path)?)?;
            }
            for rel in &self.files {
                let bytes = std::fs::read(self.world_root.join(rel))?;
                let bytes = match (strip_comments, rel.extension().and_then(|e| e.to_str()), String::from_utf8(bytes.clone())) {
                    (true, Some("md"), Ok(text)) => strip_author_comments(&text).into_bytes(),
                    _ => bytes,
                };
                let name = format!("{}/{}/{}", self.manifest.persona, self.manifest.world, rel.to_string_lossy().replace('\\', "/"));
                zip.start_file(name, options)?;
                zip.write_all(&bytes)?;
            }
            zip.finish()?;
            Ok(())
        })();
        if let Err(e) = result {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        std::fs::rename(&tmp, &self.output).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            anyhow!("can't write {}: {}", self.output.display(), e)
        })?;
        Ok(self.output.clone())
    }
}

/// What installing did.
#[derive(Debug, Clone, PartialEq)]
pub struct InstallReport {
    pub manifest: Manifest,
    /// Where the world now is.
    pub path: PathBuf,
    /// Whether an earlier install was replaced.
    pub replaced: bool,
    /// A note about the persona file, if it was written or changed.
    pub persona_note: Option<String>,
}

/// Every entry's checked path inside the package, and the manifest.
fn check_archive(archive: &mut zip::ZipArchive<std::fs::File>) -> Result<(Manifest, Vec<(usize, PathBuf)>)> {
    if archive.len() > MAX_FILES + 2 {
        bail!("the package holds {} files, over the {} allowed", archive.len(), MAX_FILES);
    }
    let manifest: Manifest = {
        let mut entry = archive.by_name(MANIFEST).map_err(|_| anyhow!("this isn't a Thwarter package: it has no {}", MANIFEST))?;
        let mut text = String::new();
        entry.by_ref().take(64 * 1024).read_to_string(&mut text)?;
        toml::from_str(&text).with_context(|| format!("its {} doesn't parse", MANIFEST))?
    };
    if manifest.format != 1 {
        bail!("this package is format {}; this thwart reads format 1", manifest.format);
    }
    for (what, id) in [("persona", &manifest.persona), ("world", &manifest.world)] {
        if !is_safe_segment(id) {
            bail!("the package's {} id `{}` can't name a folder", what, id);
        }
    }
    let world_prefix = Path::new(&manifest.persona).join(&manifest.world);
    let persona_file = Path::new(&manifest.persona).join("index.md");
    let mut total: u64 = 0;
    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i)?;
        let raw = entry.name().map_err(|e| anyhow!("an entry's name can't be read: {}", e))?.to_string();
        let shown = raw.escape_debug().to_string();
        if entry.is_symlink() {
            bail!("{} is a link, which a package may not hold", shown);
        }
        let path = entry.enclosed_name().ok_or_else(|| anyhow!("{} points outside the package", shown))?;
        // Checked on the raw name too: an absolute name (`/etc/x`, `C:x`)
        // is refused, not quietly made relative.
        let absolute = raw.starts_with('/') || raw.as_bytes().get(1) == Some(&b':');
        if absolute || raw.contains('\\') || path.components().any(|c| !matches!(c, Component::Normal(_))) {
            bail!("{} points outside the package", shown);
        }
        total += entry.size();
        if total > MAX_TOTAL_BYTES {
            bail!("the package unpacks to more than {} MB", MAX_TOTAL_BYTES / 1024 / 1024);
        }
        if entry.is_dir() || raw == MANIFEST {
            continue;
        }
        if path != persona_file && !path.starts_with(&world_prefix) {
            bail!("{} is outside {}/, where a package keeps its files", shown, world_prefix.display());
        }
        entries.push((i, path));
    }
    Ok((manifest, entries))
}

/// Install a package into `data_dir/worlds/<persona>/<world>/` (REQ-CLI-15).
/// The whole archive is checked before anything is written. An existing
/// install is refused unless `replace`, which swaps the new copy in whole.
pub fn install(package: &Path, data_dir: &Path, replace: bool) -> Result<InstallReport> {
    let file = std::fs::File::open(package).with_context(|| format!("can't open {}", package.display()))?;
    let mut archive = zip::ZipArchive::new(file).with_context(|| format!("{} isn't a zip file", package.display()))?;
    let (manifest, entries) = check_archive(&mut archive)?;

    let persona_dir = data_dir.join("worlds").join(&manifest.persona);
    let target = persona_dir.join(&manifest.world);
    let replaced = target.exists();
    if replaced && !replace {
        bail!("{}/{} is already installed; --replace swaps in this copy", manifest.persona, manifest.world);
    }
    std::fs::create_dir_all(&persona_dir).with_context(|| format!("can't create {}", persona_dir.display()))?;

    // Unpack beside the target, so a failure leaves any old copy as it was.
    let staging = persona_dir.join(format!(".{}.installing-{}", manifest.world, std::process::id()));
    let _ = std::fs::remove_dir_all(&staging);
    let world_prefix = Path::new(&manifest.persona).join(&manifest.world);
    let mut persona_text: Option<Vec<u8>> = None;
    let unpacked = (|| -> Result<()> {
        let mut budget = MAX_TOTAL_BYTES;
        for (i, path) in &entries {
            let mut entry = archive.by_index(*i)?;
            let mut bytes = Vec::new();
            // Never trust the size the header claims.
            let read = entry.by_ref().take(budget + 1).read_to_end(&mut bytes)? as u64;
            if read > budget {
                bail!("the package unpacks to more than {} MB", MAX_TOTAL_BYTES / 1024 / 1024);
            }
            budget -= read;
            match path.strip_prefix(&world_prefix) {
                Ok(inner) => {
                    let dest = staging.join(inner);
                    std::fs::create_dir_all(dest.parent().unwrap())?;
                    std::fs::write(&dest, &bytes)?;
                }
                Err(_) => persona_text = Some(bytes),
            }
        }
        std::fs::create_dir_all(&staging)?;
        std::fs::write(staging.join(INSTALLED_MANIFEST), toml::to_string_pretty(&manifest)?)?;
        Ok(())
    })();
    if let Err(e) = unpacked {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }
    if replaced {
        let old = persona_dir.join(format!(".{}.replaced-{}", manifest.world, std::process::id()));
        std::fs::rename(&target, &old).with_context(|| format!("can't move the old {} aside", target.display()))?;
        if let Err(e) = std::fs::rename(&staging, &target) {
            let _ = std::fs::rename(&old, &target);
            let _ = std::fs::remove_dir_all(&staging);
            bail!("can't put the new copy in place, so the old one stays: {}", e);
        }
        let _ = std::fs::remove_dir_all(&old);
    } else {
        std::fs::rename(&staging, &target).with_context(|| format!("can't install to {}", target.display()))?;
    }

    let persona_note = match persona_text {
        None => None,
        Some(bytes) => {
            let path = persona_dir.join("index.md");
            match std::fs::read(&path) {
                Ok(old) if old == bytes => None,
                Ok(_) => {
                    std::fs::write(&path, &bytes)?;
                    Some(format!("updated {}'s details from the package", manifest.persona))
                }
                Err(_) => {
                    std::fs::write(&path, &bytes)?;
                    Some(format!("added {}'s details", manifest.persona))
                }
            }
        }
    };
    Ok(InstallReport { manifest, path: target, replaced, persona_note })
}

/// An installed world (REQ-CLI-16).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Installed {
    pub persona: String,
    pub world: String,
    pub title: Option<String>,
    pub version: Option<String>,
    pub path: PathBuf,
}

/// Every world installed under `data_dir/worlds/`, sorted.
pub fn installed(data_dir: &Path) -> Vec<Installed> {
    let mut out = Vec::new();
    let read = |dir: &Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_dir() && p.file_name().map_or(false, |n| !n.to_string_lossy().starts_with('.'))).collect())
            .unwrap_or_default();
        v.sort();
        v
    };
    for persona in read(&data_dir.join("worlds")) {
        for world in read(&persona) {
            let manifest: Option<Manifest> = std::fs::read_to_string(world.join(INSTALLED_MANIFEST)).ok().and_then(|t| toml::from_str(&t).ok());
            out.push(Installed {
                persona: persona.file_name().unwrap().to_string_lossy().into_owned(),
                world: world.file_name().unwrap().to_string_lossy().into_owned(),
                title: manifest.as_ref().and_then(|m| m.title.clone()),
                version: manifest.as_ref().and_then(|m| m.version.clone()),
                path: world,
            });
        }
    }
    out
}

/// Which installed world `world_root` is, as `(persona, world)`, if it
/// sits at `worlds/<persona>/<world>/` in the data folder (REQ-CFG-6), so
/// its saves can follow its name rather than its path.
pub fn installed_id(world_root: &Path) -> Option<(String, String)> {
    installed_id_in(world_root, &crate::settings::data_dir()?)
}

/// `installed_id`, with the data folder given.
pub fn installed_id_in(world_root: &Path, data_dir: &Path) -> Option<(String, String)> {
    let worlds = std::fs::canonicalize(data_dir.join("worlds")).ok()?;
    let root = std::fs::canonicalize(world_root).ok()?;
    let rel = root.strip_prefix(&worlds).ok()?;
    let parts: Vec<String> = rel.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    match parts.as_slice() {
        [persona, world] if is_safe_segment(persona) && is_safe_segment(world) => Some((persona.clone(), world.clone())),
        _ => None,
    }
}

/// The installed world `persona/world` names, if there is one.
pub fn find_installed(data_dir: &Path, name: &str) -> Option<PathBuf> {
    let (persona, world) = name.split_once('/')?;
    if !is_safe_segment(persona) || !is_safe_segment(world) {
        return None;
    }
    let path = data_dir.join("worlds").join(persona).join(world);
    path.is_dir().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_installed_world_is_known_by_persona_and_world() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let world = data.join("worlds/bruce/castle");
        std::fs::create_dir_all(world.join("world")).unwrap();
        assert_eq!(installed_id_in(&world, &data), Some(("bruce".into(), "castle".into())));
        assert_eq!(installed_id_in(&world.join("world"), &data), None);
        std::fs::create_dir_all(dir.path().join("authors/bruce/castle")).unwrap();
        assert_eq!(installed_id_in(&dir.path().join("authors/bruce/castle"), &data), None);
        assert_eq!(find_installed(&data, "bruce/castle"), Some(world));
        assert_eq!(find_installed(&data, "../etc"), None);
    }

    #[test]
    fn author_comments_strip_but_markers_stay() {
        let text = "A hall. <!-- inline -->\n<!-- alone -->\n<!-- thwarter:action pull -->\nClunk.\n";
        assert_eq!(strip_author_comments(text), "A hall. \n<!-- thwarter:action pull -->\nClunk.\n");
        assert_eq!(comment_spans(text).len(), 2);
    }
}
