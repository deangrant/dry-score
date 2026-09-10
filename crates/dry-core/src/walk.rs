//! Filesystem discovery of source files.
//!
//! Symlink entries (file or directory) are not followed, so discovery stays on
//! the lexical tree under each analysis root.

use std::fs;
use std::path::{Path, PathBuf};

/// Options controlling recursive source discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkOptions {
    /// Extensions without the leading dot.
    pub extensions: Vec<String>,
    /// Path component names to skip (matched relative to each walk root).
    pub exclude: Vec<String>,
}

impl WalkOptions {
    /// Builds walk options from extension and exclude lists.
    #[must_use]
    pub const fn new(extensions: Vec<String>, exclude: Vec<String>) -> Self {
        Self {
            extensions,
            exclude,
        }
    }
}

/// Errors from walking the filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkError {
    /// Human-readable explanation.
    pub message: String,
}

impl WalkError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for WalkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for WalkError {}

/// Recursively collects source files under `roots`.
///
/// Exclude names match path components **relative to each root**, so a root
/// that itself lives under an excluded name (e.g. a `fixtures/` corpus) is
/// still analyzed when passed as an explicit root.
///
/// Symlinks are not followed (neither files nor directories), which keeps the
/// walk inside the lexical tree and avoids symlink cycles.
///
/// # Errors
///
/// Returns [`WalkError`] when a root cannot be read.
pub fn collect_source_files(
    roots: &[PathBuf],
    options: &WalkOptions,
) -> Result<Vec<PathBuf>, WalkError> {
    let mut files = Vec::new();
    for root in roots {
        collect_into(root, root, options, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_into(
    path: &Path,
    root: &Path,
    options: &WalkOptions,
    out: &mut Vec<PathBuf>,
) -> Result<(), WalkError> {
    if is_excluded_relative(path, root, &options.exclude) {
        return Ok(());
    }
    let meta = fs::symlink_metadata(path)
        .map_err(|err| WalkError::new(format!("failed to stat {}: {err}", path.display())))?;
    collect_meta(path, root, options, &meta, out)
}

fn collect_meta(
    path: &Path,
    root: &Path,
    options: &WalkOptions,
    meta: &fs::Metadata,
    out: &mut Vec<PathBuf>,
) -> Result<(), WalkError> {
    if meta.file_type().is_symlink() {
        return Ok(());
    }
    if meta.is_file() {
        collect_file(path, options, out);
        return Ok(());
    }
    if meta.is_dir() {
        return collect_dir(path, root, options, out);
    }
    Ok(())
}

fn collect_file(path: &Path, options: &WalkOptions, out: &mut Vec<PathBuf>) {
    if has_extension(path, &options.extensions) {
        out.push(path.to_path_buf());
    }
}

fn collect_dir(
    path: &Path,
    root: &Path,
    options: &WalkOptions,
    out: &mut Vec<PathBuf>,
) -> Result<(), WalkError> {
    for entry in read_dir_entries(path)? {
        collect_into(&entry.path(), root, options, out)?;
    }
    Ok(())
}

fn read_dir_entries(path: &Path) -> Result<Vec<fs::DirEntry>, WalkError> {
    let entries = fs::read_dir(path).map_err(|err| walk_dir_error(path, &err))?;
    let mut out = Vec::new();
    for entry in entries {
        out.push(entry.map_err(|err| walk_entry_error(path, &err))?);
    }
    Ok(out)
}

fn walk_dir_error(path: &Path, err: &std::io::Error) -> WalkError {
    WalkError::new(format!("failed to read {}: {err}", path.display()))
}

fn walk_entry_error(path: &Path, err: &std::io::Error) -> WalkError {
    WalkError::new(format!(
        "failed to read entry under {}: {err}",
        path.display()
    ))
}

fn is_excluded_relative(path: &Path, root: &Path, excludes: &[String]) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    if relative.as_os_str().is_empty() {
        return false;
    }
    relative.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        excludes.iter().any(|ex| name.as_ref() == ex)
    })
}

fn has_extension(path: &Path, extensions: &[String]) -> bool {
    let Some(ext) = path.extension() else {
        return false;
    };
    let ext = ext.to_string_lossy();
    extensions.iter().any(|wanted| wanted == ext.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn exclude_matches_relative_component() {
        let root = Path::new("repo");
        let nested = Path::new("repo/crates/foo/target/debug/x.rs");
        assert!(is_excluded_relative(nested, root, &["target".to_owned()]));
    }

    #[test]
    fn root_under_excluded_name_is_not_skipped() {
        let root = Path::new("crates/dry-rs/tests/fixtures/type_1_exact");
        let file = root.join("main.rs");
        assert!(!is_excluded_relative(&file, root, &["fixtures".to_owned()]));
    }

    fn temp_project() -> PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let base = std::env::temp_dir().join(format!("dry-rs-walk-{stamp}"));
        assert!(fs::create_dir_all(base.join("src")).is_ok());
        assert!(fs::create_dir_all(base.join("fixtures")).is_ok());
        assert!(fs::write(base.join("src/lib.rs"), "fn a() {}\n").is_ok());
        assert!(fs::write(base.join("fixtures/dup.rs"), "fn b() {}\n").is_ok());
        base
    }

    fn assert_walk_len(root: &Path, options: &WalkOptions, expected: usize) {
        let files = collect_source_files(std::slice::from_ref(&root.to_path_buf()), options);
        assert!(files.is_ok());
        #[expect(clippy::expect_used, reason = "test asserts walk succeeded above")]
        let files = files.expect("walk ok");
        assert_eq!(files.len(), expected);
    }

    #[test]
    fn repo_walk_skips_fixtures_subdir() {
        let base = temp_project();
        let options = WalkOptions::new(vec!["rs".to_owned()], vec!["fixtures".to_owned()]);
        assert_walk_len(&base, &options, 1);
        assert_walk_len(&base.join("fixtures"), &options, 1);
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn walk_missing_root_errors() {
        let options = WalkOptions::new(vec!["rs".to_owned()], Vec::new());
        let missing = PathBuf::from("/no/such/dry-rs-walk-root");
        let err = collect_source_files(&[missing], &options);
        assert!(err.is_err());
        #[expect(clippy::expect_used, reason = "test asserts walk error")]
        let err = err.expect_err("missing");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn has_extension_requires_suffix() {
        assert!(!has_extension(Path::new("Makefile"), &["rs".to_owned()]));
        assert!(has_extension(Path::new("lib.rs"), &["rs".to_owned()]));
    }

    #[test]
    fn walk_skips_non_matching_extension() {
        let base = temp_project();
        assert!(fs::write(base.join("src/notes.txt"), "x\n").is_ok());
        let options = WalkOptions::new(vec!["rs".to_owned()], Vec::new());
        assert_walk_len(&base, &options, 2);
        let _ = fs::remove_dir_all(base);
    }

    #[cfg(unix)]
    #[test]
    fn walk_skips_socket_nodes() {
        use std::os::unix::net::UnixListener;
        let base = temp_project();
        let sock = base.join("s.sock");
        let _listener = UnixListener::bind(&sock);
        let options = WalkOptions::new(vec!["rs".to_owned()], Vec::new());
        assert_walk_len(&base, &options, 2);
        let _ = fs::remove_dir_all(base);
    }

    #[cfg(unix)]
    #[test]
    fn walk_skips_directory_symlink_escape() {
        use std::os::unix::fs::symlink;
        let base = temp_project();
        let outside = std::env::temp_dir().join(format!(
            "dry-rs-walk-outside-{}",
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
        ));
        assert!(fs::create_dir_all(&outside).is_ok());
        assert!(fs::write(outside.join("secret.rs"), "fn leak() {}\n").is_ok());
        assert!(symlink(&outside, base.join("out")).is_ok());
        let options = WalkOptions::new(vec!["rs".to_owned()], Vec::new());
        assert_walk_len(&base, &options, 2);
        let _ = fs::remove_dir_all(outside);
        let _ = fs::remove_dir_all(base);
    }

    #[cfg(unix)]
    #[test]
    fn walk_skips_file_symlink() {
        use std::os::unix::fs::symlink;
        let base = temp_project();
        let outside = std::env::temp_dir().join(format!(
            "dry-rs-walk-file-{}",
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
        ));
        assert!(fs::write(&outside, "fn leak() {}\n").is_ok());
        assert!(symlink(&outside, base.join("src/link.rs")).is_ok());
        let options = WalkOptions::new(vec!["rs".to_owned()], Vec::new());
        assert_walk_len(&base, &options, 2);
        let _ = fs::remove_file(outside);
        let _ = fs::remove_dir_all(base);
    }

    #[cfg(unix)]
    #[test]
    fn walk_skips_symlink_cycle() {
        use std::os::unix::fs::symlink;
        let base = temp_project();
        assert!(symlink(".", base.join("loop")).is_ok());
        let options = WalkOptions::new(vec!["rs".to_owned()], Vec::new());
        assert_walk_len(&base, &options, 2);
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn read_dir_helpers_cover_errors() {
        let missing = Path::new("/no/such/dry-rs-dir");
        assert!(read_dir_entries(missing).is_err());
        let err = std::io::Error::other("boom");
        assert!(!walk_dir_error(missing, &err).to_string().is_empty());
        assert!(!walk_entry_error(missing, &err).to_string().is_empty());
    }
}
