//! Same-directory atomic replacement preserving existing file permissions.
//! Modified for Ariember/fastctx: retain the file-edit write path.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Returns the Unix mode of an existing file, or `None` on other platforms.
pub(super) fn existing_unix_mode(path: &Path) -> Option<u32> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .ok()
            .map(|metadata| metadata.permissions().mode())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

/// Writes a temporary file beside the target and atomically replaces the target.
pub(super) fn atomic_replace(
    target: &Path,
    bytes: &[u8],
    unix_mode: Option<u32>,
) -> Result<(), String> {
    let parent = target.parent().ok_or_else(|| {
        format!(
            "Cannot determine the parent directory for {}.",
            crate::paths::display_path(target)
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Cannot create {}: {error}",
            crate::paths::display_path(parent)
        )
    })?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let filename = target
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("fastctx");
    let temporary = parent.join(format!(
        ".{filename}.{}.{}.tmp",
        std::process::id(),
        sequence
    ));
    if let Err(error) = write_temporary(&temporary, bytes, unix_mode) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = replace_path(&temporary, target) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

fn write_temporary(path: &Path, bytes: &[u8], unix_mode: Option<u32>) -> Result<(), String> {
    #[cfg(not(unix))]
    let _ = unix_mode;
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(unix_mode.unwrap_or(0o600));
    }
    let mut file = options.open(path).map_err(|error| {
        format!(
            "Cannot create temporary file {}: {error}",
            crate::paths::display_path(path)
        )
    })?;
    #[cfg(unix)]
    if let Some(mode) = unix_mode {
        use std::os::unix::fs::PermissionsExt;
        // OpenOptionsExt::mode is filtered by umask; restore the existing mode before publish.
        file.set_permissions(fs::Permissions::from_mode(mode))
            .map_err(|error| {
                format!(
                    "Cannot preserve permissions on temporary file {}: {error}",
                    crate::paths::display_path(path)
                )
            })?;
    }
    file.write_all(bytes).map_err(|error| {
        format!(
            "Cannot write temporary file {}: {error}",
            crate::paths::display_path(path)
        )
    })?;
    file.sync_all().map_err(|error| {
        format!(
            "Cannot sync temporary file {}: {error}",
            crate::paths::display_path(path)
        )
    })
}

#[cfg(unix)]
fn replace_path(temporary: &Path, target: &Path) -> Result<(), String> {
    fs::rename(temporary, target).map_err(|error| {
        format!(
            "Cannot atomically replace {}: {error}",
            crate::paths::display_path(target)
        )
    })
}

#[cfg(windows)]
fn replace_path(temporary: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_WRITE_THROUGH, MoveFileExW, REPLACEFILE_WRITE_THROUGH, ReplaceFileW,
    };

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    let temporary_wide = wide(temporary);
    let target_wide = wide(target);
    if !target.exists() {
        let moved = unsafe {
            MoveFileExW(
                temporary_wide.as_ptr(),
                target_wide.as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            )
        };
        if moved != 0 {
            return Ok(());
        }
        return Err(format!(
            "Cannot atomically create {}: {}",
            crate::paths::display_path(target),
            std::io::Error::last_os_error()
        ));
    }
    let replaced = unsafe {
        ReplaceFileW(
            target_wide.as_ptr(),
            temporary_wide.as_ptr(),
            std::ptr::null(),
            REPLACEFILE_WRITE_THROUGH,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced != 0 {
        return Ok(());
    }
    Err(format!(
        "Cannot atomically replace {}: {}",
        crate::paths::display_path(target),
        std::io::Error::last_os_error()
    ))
}

#[cfg(test)]
mod tests {
    use super::{atomic_replace, existing_unix_mode};

    #[test]
    fn replacement_preserves_bytes_and_existing_permissions() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("file.txt");
        std::fs::write(&path, b"old").unwrap();
        let mode = existing_unix_mode(&path);
        atomic_replace(&path, b"new\r\n", mode).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new\r\n");
        assert_eq!(existing_unix_mode(&path), mode);
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    }
}
