//! Contracts for explicit filesystem mutations through the native tool router.

mod common;

use common::{call_tool, normalized, tool_text, write};
use serde_json::json;
use std::fs;

#[test]
fn delete_defaults_to_preview_and_rejects_non_empty_directories() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("file.txt");
    let empty = temp.path().join("empty");
    let tree = temp.path().join("tree");
    write(&file, b"keep");
    fs::create_dir(&empty).unwrap();
    write(&tree.join("child.txt"), b"keep child");

    let response = call_tool(
        temp.path(),
        8500,
        "delete",
        json!({
            "paths": [normalized(&file), normalized(&empty), normalized(&tree)],
        }),
    );

    assert_ne!(response["isError"], true);
    let report = tool_text(&response);
    assert!(report.contains("would delete"), "{report}");
    assert!(report.contains("Directory is not empty"), "{report}");
    assert!(
        report.ends_with("(Partial: dry run; 2 entries would be deleted; 1 failed.)"),
        "{report}"
    );
    assert_eq!(fs::read(&file).unwrap(), b"keep");
    assert!(empty.is_dir());
    assert_eq!(fs::read(tree.join("child.txt")).unwrap(), b"keep child");
}

#[test]
fn delete_keeps_request_order_and_completed_results_on_partial_failure() {
    let temp = tempfile::tempdir().unwrap();
    let binary = temp.path().join("first.bin");
    let missing = temp.path().join("missing.txt");
    let empty = temp.path().join("last");
    write(&binary, [0, 255, 0, 128]);
    fs::create_dir(&empty).unwrap();

    let response = call_tool(
        temp.path(),
        8500,
        "delete",
        json!({
            "paths": [normalized(&binary), normalized(&missing), normalized(&empty)],
            "dry_run": false,
        }),
    );

    assert_ne!(response["isError"], true);
    let report = tool_text(&response);
    assert!(report.find("first.bin").unwrap() < report.find("missing.txt").unwrap());
    assert!(report.find("missing.txt").unwrap() < report.find("last:").unwrap());
    assert!(
        report.ends_with("(Partial: 2 entries deleted; 1 failed.)"),
        "{report}"
    );
    assert!(!binary.exists());
    assert!(!empty.exists());
}

#[test]
fn delete_only_removes_explicit_entries_and_does_not_expand_patterns() {
    let temp = tempfile::tempdir().unwrap();
    let explicit = temp.path().join("file[1].txt");
    let other = temp.path().join("file1.txt");
    write(&explicit, b"explicit");
    write(&other, b"other");

    let response = call_tool(
        temp.path(),
        8500,
        "delete",
        json!({
            "paths": [normalized(&explicit), "file1.txt"],
            "dry_run": false,
        }),
    );

    assert!(tool_text(&response).contains("Expected an absolute filesystem path"));
    assert!(!explicit.exists());
    assert_eq!(fs::read(&other).unwrap(), b"other");

    let response = call_tool(temp.path(), 8500, "delete", json!({"paths": []}));
    assert_eq!(response["isError"], true);
}

#[test]
fn mutations_refuse_to_execute_when_the_status_cannot_fit_the_budget() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("keep.txt");
    write(&file, b"keep");
    let response = call_tool(
        temp.path(),
        1,
        "delete",
        json!({
            "paths": [normalized(&file)], "dry_run": false,
        }),
    );
    assert_eq!(response["isError"], true);
    assert_eq!(fs::read(&file).unwrap(), b"keep");
    let destination = temp.path().join("new.txt");
    for (name, arguments) in [
        (
            "move",
            json!({"source": normalized(&file), "destination": normalized(&destination)}),
        ),
        (
            "create_file",
            json!({"path": normalized(&destination), "content": "new"}),
        ),
    ] {
        let response = call_tool(temp.path(), 1, name, arguments);
        assert_eq!(response["isError"], true);
        assert_eq!(fs::read(&file).unwrap(), b"keep");
        assert!(!destination.exists());
    }
}

#[test]
fn truncated_delete_reports_retain_actual_totals_and_stay_within_budget() {
    let temp = tempfile::tempdir().unwrap();
    let mut paths = Vec::new();
    for index in 0..8 {
        let path = temp
            .path()
            .join(format!("{index}-{}.txt", "long-name-".repeat(8)));
        write(&path, b"delete");
        paths.push(normalized(&path));
    }
    paths.push(normalized(&temp.path().join("missing.txt")));
    let response = call_tool(
        temp.path(),
        128,
        "delete",
        json!({
            "paths": paths, "dry_run": false,
        }),
    );
    assert_ne!(response["isError"], true);
    let report = tool_text(&response);
    assert!(report.contains("totals below cover all files"), "{report}");
    assert!(
        report.ends_with(
            "(Partial: 8 entries deleted; 1 failed; list truncated, see the note above.)"
        ),
        "{report}"
    );
    assert!(fastctx::budget::estimate_tokens(report) <= 128);
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn move_renames_files_and_non_empty_directories_without_overwriting() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.bin");
    let destination = temp.path().join("destination.bin");
    write(&source, [0, 255, 128]);
    let response = call_tool(
        temp.path(),
        8500,
        "move",
        json!({
            "source": normalized(&source), "destination": normalized(&destination),
        }),
    );
    assert_ne!(response["isError"], true);
    assert!(tool_text(&response).ends_with("(Complete: 1 entry moved.)"));
    assert!(!source.exists());
    assert_eq!(fs::read(&destination).unwrap(), [0, 255, 128]);

    write(&source, b"source");
    let response = call_tool(
        temp.path(),
        8500,
        "move",
        json!({
            "source": normalized(&source), "destination": normalized(&destination),
        }),
    );
    assert_eq!(response["isError"], true);
    assert!(tool_text(&response).contains("already exists"));
    assert_eq!(fs::read(&source).unwrap(), b"source");
    assert_eq!(fs::read(&destination).unwrap(), [0, 255, 128]);

    let directory = temp.path().join("directory");
    let renamed = temp.path().join("renamed");
    write(&directory.join("child.txt"), b"child");
    let response = call_tool(
        temp.path(),
        8500,
        "move",
        json!({
            "source": normalized(&directory), "destination": normalized(&renamed),
        }),
    );
    assert_ne!(response["isError"], true);
    assert!(!directory.exists());
    assert_eq!(fs::read(renamed.join("child.txt")).unwrap(), b"child");
}

#[test]
fn create_file_preserves_utf8_content_and_refuses_existing_entries() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("new.txt");
    let content = "中文\r\ntext without final newline";
    let response = call_tool(
        temp.path(),
        8500,
        "create_file",
        json!({
            "path": normalized(&file), "content": content,
        }),
    );
    assert_ne!(response["isError"], true);
    assert!(tool_text(&response).ends_with("(Complete: 1 file created.)"));
    assert_eq!(fs::read(&file).unwrap(), content.as_bytes());

    for path in [&file, temp.path()] {
        let response = call_tool(
            temp.path(),
            8500,
            "create_file",
            json!({
                "path": normalized(path), "content": "overwrite",
            }),
        );
        assert_eq!(response["isError"], true);
    }
    assert_eq!(fs::read(&file).unwrap(), content.as_bytes());
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);

    let empty = temp.path().join("empty.txt");
    let response = call_tool(
        temp.path(),
        8500,
        "create_file",
        json!({
            "path": normalized(&empty), "content": "",
        }),
    );
    assert_ne!(response["isError"], true);
    assert!(fs::read(empty).unwrap().is_empty());
}

#[test]
fn move_and_create_require_absolute_paths_and_existing_parents() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.txt");
    write(&source, b"keep");
    for destination in [
        "relative.txt".to_string(),
        normalized(&temp.path().join("missing/new.txt")),
    ] {
        let response = call_tool(
            temp.path(),
            8500,
            "create_file",
            json!({
                "path": destination, "content": "new",
            }),
        );
        assert_eq!(response["isError"], true);
        let response = call_tool(
            temp.path(),
            8500,
            "move",
            json!({
                "source": normalized(&source), "destination": destination,
            }),
        );
        assert_eq!(response["isError"], true);
    }
    assert_eq!(fs::read(source).unwrap(), b"keep");
    assert!(!temp.path().join("missing").exists());
}

#[cfg(any(unix, windows))]
#[test]
fn symlink_entries_are_removed_or_moved_without_touching_their_targets() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target.txt");
    let link = temp.path().join("link.txt");
    let moved = temp.path().join("moved.txt");
    write(&target, b"keep target");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &link).unwrap();
    #[cfg(windows)]
    if let Err(error) = std::os::windows::fs::symlink_file(&target, &link) {
        if error.raw_os_error() == Some(1314) {
            eprintln!("symlink test requires the Windows symbolic link privilege");
            return;
        }
        panic!("cannot create test symlink: {error}");
    }

    // Use the link spelling: normalized() canonicalizes existing symbolic links.
    let link_path = link.to_str().unwrap();
    for (name, args) in [
        (
            "create_file",
            json!({"path": link_path, "content": "overwrite"}),
        ),
        (
            "move",
            json!({"source": normalized(&target), "destination": link_path}),
        ),
    ] {
        assert_eq!(call_tool(temp.path(), 8500, name, args)["isError"], true);
    }
    let response = call_tool(
        temp.path(),
        8500,
        "move",
        json!({
            "source": link_path, "destination": moved.to_str().unwrap(),
        }),
    );
    assert_ne!(response["isError"], true);
    assert!(
        fs::symlink_metadata(&moved)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(&target).unwrap(), b"keep target");

    let response = call_tool(
        temp.path(),
        8500,
        "delete",
        json!({
            "paths": [moved.to_str().unwrap()], "dry_run": false,
        }),
    );
    assert_ne!(response["isError"], true);
    assert!(fs::symlink_metadata(&moved).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"keep target");

    #[cfg(unix)]
    std::os::unix::fs::symlink(temp.path().join("absent"), &link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(temp.path().join("absent"), &link).unwrap();
    let response = call_tool(
        temp.path(),
        8500,
        "create_file",
        json!({
            "path": link_path, "content": "overwrite dangling link",
        }),
    );
    assert_eq!(response["isError"], true);
    let response = call_tool(
        temp.path(),
        8500,
        "delete",
        json!({
            "paths": [link_path], "dry_run": false,
        }),
    );
    assert_ne!(response["isError"], true);
    assert!(fs::symlink_metadata(&link).is_err());

    let directory = temp.path().join("directory");
    let directory_link = temp.path().join("directory-link");
    write(&directory.join("child.txt"), b"keep child");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&directory, &directory_link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&directory, &directory_link).unwrap();
    let response = call_tool(
        temp.path(),
        8500,
        "delete",
        json!({
            "paths": [directory_link.to_str().unwrap()], "dry_run": false,
        }),
    );
    assert_ne!(response["isError"], true);
    assert!(fs::symlink_metadata(&directory_link).is_err());
    assert_eq!(
        fs::read(directory.join("child.txt")).unwrap(),
        b"keep child"
    );
}
