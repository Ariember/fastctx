//! Explicit filesystem entry operations sharing the replacement service's locks.

use super::locks::{FilePathLock, PathIdentity};
use super::replace::{ReportGroup, render_report};
use super::{ReplaceService, edit_token_budget};
use crate::budget::estimate_tokens;
use crate::model::ToolResponse;
use crate::path_codec::{display_path, parse_input_path};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Parameters for deleting explicit filesystem entries in request order.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
pub struct DeleteRequest {
    /// Explicit absolute paths; no glob expansion or ignore filtering is applied.
    #[schemars(description = crate::model_guidance::local_path_description(
        "Explicit paths to files, empty directories, or symbolic links; no glob expansion."
    ))]
    pub paths: Vec<String>,
    /// Defaults to true. Set false explicitly to delete the listed entries.
    pub dry_run: Option<bool>,
}

/// Parameters for moving or renaming one filesystem entry.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
pub struct MoveRequest {
    #[schemars(description = crate::model_guidance::local_path_description(
        "Existing file, directory, or symbolic link to move."
    ))]
    pub source: String,
    #[schemars(description = crate::model_guidance::local_path_description(
        "Destination entry; must not exist and its parent directory must exist."
    ))]
    pub destination: String,
}

/// Parameters for creating one UTF-8 text file.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
pub struct CreateFileRequest {
    #[schemars(description = crate::model_guidance::local_path_description(
        "New file path; must not exist and its parent directory must exist."
    ))]
    pub path: String,
    /// UTF-8 contents, written verbatim without adding a BOM or newline.
    pub content: String,
}

pub(super) fn delete(editor: &ReplaceService, request: DeleteRequest) -> ToolResponse {
    if request.paths.is_empty() {
        return ToolResponse::error("The paths parameter must contain at least one absolute path.");
    }
    let dry_run = request.dry_run.unwrap_or(true);
    let budget = match edit_token_budget() {
        Ok(budget) => budget,
        Err(error) => return ToolResponse::error(error),
    };
    let maximum_status = format!(
        "(Note: showing {} of {} files; totals below cover all files.)\n\n\
         (Partial: dry run; {} entries would be deleted; {} failed; list truncated, see the note above.)",
        request.paths.len(),
        request.paths.len(),
        request.paths.len(),
        request.paths.len()
    );
    if let Err(error) = check_report_budget(&maximum_status, budget) {
        return ToolResponse::error(error);
    }
    let mut lines = Vec::new();
    let mut completed = 0;
    let mut failed = 0;
    for input in &request.paths {
        let result = resolve_entry(input).and_then(|path| {
            with_entry_locks(editor, &[&path], || {
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|error| operation_error("inspect", &path, &error))?;
                if metadata.is_dir() && !metadata.file_type().is_symlink() {
                    let mut entries = fs::read_dir(&path)
                        .map_err(|error| operation_error("list", &path, &error))?;
                    if let Some(entry) = entries.next() {
                        entry.map_err(|error| operation_error("list", &path, &error))?;
                        return Err(format!(
                            "Directory is not empty: {}. Recursive deletion is not supported.",
                            display_path(&path)
                        ));
                    }
                }
                if !dry_run {
                    remove_entry(&path, &metadata)
                        .map_err(|error| operation_error("delete", &path, &error))?;
                }
                Ok(display_path(&path))
            })
        });
        match result {
            Ok(path) => {
                completed += 1;
                let action = if dry_run { "would delete" } else { "deleted" };
                lines.push(format!("{path}: {action}"));
            }
            Err(error) => {
                failed += 1;
                lines.push(format!("{}: failed: {error}", quoted_input(input)));
            }
        }
    }
    let status = if failed == 0 { "Complete" } else { "Partial" };
    let action = if dry_run {
        "would be deleted"
    } else {
        "deleted"
    };
    let preview = if dry_run { "dry run; " } else { "" };
    let terminal = format!("({status}: {preview}{completed} entries {action}; {failed} failed.)");
    let groups = lines
        .into_iter()
        .map(|line| ReportGroup { lines: vec![line] })
        .collect::<Vec<_>>();
    render_report(&groups, &terminal, &[], budget, false)
}

pub(super) fn move_path(editor: &ReplaceService, request: MoveRequest) -> ToolResponse {
    let result = (|| {
        let budget = edit_token_budget()?;
        let source = resolve_entry(&request.source)?;
        let destination = resolve_entry(&request.destination)?;
        let report = format!(
            "{} -> {}\n(Complete: 1 entry moved.)",
            display_path(&source),
            display_path(&destination)
        );
        check_report_budget(&report, budget)?;
        with_entry_locks(editor, &[&source, &destination], || {
            fs::symlink_metadata(&source)
                .map_err(|error| operation_error("inspect", &source, &error))?;
            require_missing(&destination)?;
            fs::rename(&source, &destination).map_err(|error| {
                format!(
                    "Cannot move {} to {}: {error}. Moves require the same filesystem.",
                    display_path(&source),
                    display_path(&destination)
                )
            })?;
            Ok(report)
        })
    })();
    match result {
        Ok(report) => ToolResponse::text(report),
        Err(error) => ToolResponse::error(error),
    }
}

pub(super) fn create_file(editor: &ReplaceService, request: CreateFileRequest) -> ToolResponse {
    let result = (|| {
        let budget = edit_token_budget()?;
        let path = resolve_entry(&request.path)?;
        let report = format!(
            "{}: {} bytes\n(Complete: 1 file created.)",
            display_path(&path),
            request.content.len()
        );
        check_report_budget(&report, budget)?;
        with_entry_locks(editor, &[&path], || {
            require_missing(&path)?;
            let mut temporary = tempfile::NamedTempFile::new_in(path.parent().unwrap())
                .map_err(|error| operation_error("create temporary file for", &path, &error))?;
            temporary
                .write_all(request.content.as_bytes())
                .and_then(|()| temporary.as_file().sync_all())
                .map_err(|error| operation_error("write", &path, &error))?;
            temporary
                .persist_noclobber(&path)
                .map_err(|error| operation_error("create", &path, &error.error))?;
            Ok(report)
        })
    })();
    match result {
        Ok(report) => ToolResponse::text(report),
        Err(error) => ToolResponse::error(error),
    }
}

fn resolve_entry(input: &str) -> Result<PathBuf, String> {
    let path = parse_input_path(input)?;
    if !path.is_absolute() {
        return Err(format!(
            "Expected an absolute filesystem path: {}.",
            quoted_input(input)
        ));
    }
    let name = path.file_name().ok_or_else(|| {
        format!(
            "Expected a named filesystem entry: {}.",
            display_path(&path)
        )
    })?;
    let parent = crate::paths::canonical_existing(path.parent().unwrap())
        .map_err(|error| operation_error("resolve parent of", &path, &error))?;
    if !parent.is_dir() {
        return Err(format!(
            "Parent is not a directory: {}.",
            display_path(&parent)
        ));
    }
    Ok(parent.join(name))
}

fn require_missing(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(format!(
            "Destination already exists: {}.",
            display_path(path)
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(operation_error("inspect", path, &error)),
    }
}

fn check_report_budget(report: &str, budget: usize) -> Result<(), String> {
    if estimate_tokens(report) > budget {
        return Err(format!(
            "FASTCTX_TOKEN_BUDGET={budget} is too small to report mutation results. Increase it and retry."
        ));
    }
    Ok(())
}

fn with_entry_locks<T>(
    editor: &ReplaceService,
    paths: &[&Path],
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let mut names = BTreeMap::new();
    for path in paths {
        names.insert(PathIdentity::for_name(path)?, *path);
    }
    let name_locks = names
        .keys()
        .map(|key| editor.path_locks.for_identity(key))
        .collect::<Vec<_>>();
    let _name_guards = name_locks
        .iter()
        .map(|lock| lock.lock().unwrap())
        .collect::<Vec<_>>();
    let _name_file_guards = names
        .iter()
        .map(|(key, path)| FilePathLock::acquire(key, path))
        .collect::<Result<Vec<_>, _>>()?;

    let mut targets = BTreeMap::new();
    for path in paths {
        match fs::symlink_metadata(path) {
            Ok(metadata) if !metadata.file_type().is_symlink() => {
                targets.insert(PathIdentity::for_existing(path)?, *path);
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(operation_error("inspect", path, &error)),
        }
    }
    let target_locks = targets
        .keys()
        .map(|key| editor.path_locks.for_identity(key))
        .collect::<Vec<_>>();
    let _target_guards = target_locks
        .iter()
        .map(|lock| lock.lock().unwrap())
        .collect::<Vec<_>>();
    let _target_file_guards = targets
        .iter()
        .map(|(key, path)| FilePathLock::acquire(key, path))
        .collect::<Result<Vec<_>, _>>()?;
    operation()
}

fn remove_entry(path: &Path, metadata: &fs::Metadata) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        if metadata.file_type().is_symlink_dir() {
            return fs::remove_dir(path);
        }
    }
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    }
}

fn operation_error(action: &str, path: &Path, error: &io::Error) -> String {
    format!("Cannot {action} {}: {error}", display_path(path))
}

fn quoted_input(input: &str) -> String {
    serde_json::to_string(input).expect("strings are JSON serializable")
}

#[cfg(test)]
mod tests {
    use super::{PathIdentity, ReplaceService, with_entry_locks};

    #[test]
    fn entry_operations_hold_the_same_name_and_target_locks_as_replace() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let destination = temp.path().join("destination.txt");
        std::fs::write(&source, b"source").unwrap();
        let editor = ReplaceService::new();
        with_entry_locks(&editor, &[&source, &destination, &source], || {
            for identity in [
                PathIdentity::for_name(&source)?,
                PathIdentity::for_name(&destination)?,
                PathIdentity::for_existing(&source)?,
            ] {
                let lock = editor.path_locks.for_identity(&identity);
                assert!(lock.try_lock().is_err());
            }
            Ok(())
        })
        .unwrap();
    }
}
