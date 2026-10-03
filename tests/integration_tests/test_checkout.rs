use crate::common::{run_grit_command, setup_integration_test};
use std::fs;

#[test]
fn test_checkout_workflow() {
    let test_dir = setup_integration_test();

    // Initialize repository
    run_grit_command(&test_dir, &["init"]).unwrap();

    // Create files and commit
    fs::write(test_dir.path().join("original.txt"), "Original content").unwrap();
    fs::create_dir(test_dir.path().join("subdir")).unwrap();
    fs::write(
        test_dir.path().join("subdir").join("nested.txt"),
        "Nested content",
    )
    .unwrap();

    run_grit_command(&test_dir, &["add", "."]).unwrap();

    let commit_result = run_grit_command(&test_dir, &["commit", "--message", "Initial commit"]);
    assert!(commit_result.is_ok());
    let commit_hash = commit_result.unwrap();

    // Modify files
    fs::write(test_dir.path().join("original.txt"), "Modified content").unwrap();
    fs::remove_file(test_dir.path().join("subdir").join("nested.txt")).unwrap();

    // Checkout the commit
    let checkout_result = run_grit_command(&test_dir, &["checkout", &commit_hash]);
    assert!(checkout_result.is_ok());

    // Verify files were restored
    assert_eq!(
        fs::read_to_string(test_dir.path().join("original.txt")).unwrap(),
        "Original content"
    );
    assert!(test_dir.path().join("subdir").join("nested.txt").exists());
    assert_eq!(
        fs::read_to_string(test_dir.path().join("subdir").join("nested.txt")).unwrap(),
        "Nested content"
    );
}

#[test]
fn test_checkout_tree_snapshot() {
    let test_dir = setup_integration_test();

    run_grit_command(&test_dir, &["init"]).unwrap();

    fs::write(test_dir.path().join("snap.txt"), "snapshot content\n").unwrap();
    run_grit_command(&test_dir, &["add", "snap.txt"]).unwrap();
    let tree_hash = run_grit_command(&test_dir, &["write-tree"]).unwrap();

    // Modify file
    fs::write(test_dir.path().join("snap.txt"), "modified content\n").unwrap();

    // Checkout tree snapshot directly
    let checkout_out = run_grit_command(&test_dir, &["checkout", &tree_hash]).unwrap();
    assert!(checkout_out.contains("Restored snapshot"));

    // File restored to snapshot content
    assert_eq!(
        fs::read_to_string(test_dir.path().join("snap.txt")).unwrap(),
        "snapshot content\n"
    );
}

#[test]
fn test_checkout_error_cases() {
    let test_dir = setup_integration_test();

    run_grit_command(&test_dir, &["init"]).unwrap();

    // 1. Cannot checkout -b on empty repo
    let err_empty_b = run_grit_command(&test_dir, &["checkout", "-b", "dev"]);
    assert!(err_empty_b.is_err());
    assert!(
        err_empty_b
            .unwrap_err()
            .contains("not a valid object name: 'HEAD'")
    );

    // Initial commit
    fs::write(test_dir.path().join("file.txt"), "content").unwrap();
    run_grit_command(&test_dir, &["add", "."]).unwrap();
    run_grit_command(&test_dir, &["commit", "-m", "First"]).unwrap();

    // 2. Cannot checkout nonexistent branch/pathspec
    let err_nonexistent = run_grit_command(&test_dir, &["checkout", "ghost_branch"]);
    assert!(err_nonexistent.is_err());
    assert!(
        err_nonexistent
            .unwrap_err()
            .contains("did not match any file(s) known to grit")
    );

    // 3. Cannot checkout -b with name of already existing branch
    let err_existing = run_grit_command(&test_dir, &["checkout", "-b", "main"]);
    assert!(err_existing.is_err());
    assert!(err_existing.unwrap_err().contains("already exists"));
}

#[test]
fn test_checkout_branch_workflow() {
    let test_dir = setup_integration_test();

    run_grit_command(&test_dir, &["init"]).unwrap();
    fs::write(test_dir.path().join("base.txt"), "base content").unwrap();
    run_grit_command(&test_dir, &["add", "."]).unwrap();
    run_grit_command(&test_dir, &["commit", "-m", "Main commit"]).unwrap();

    // Create and switch to new branch dev
    let out = run_grit_command(&test_dir, &["checkout", "-b", "dev"]).unwrap();
    assert!(out.contains("Switched to a new branch 'dev'"));

    // Verify branch list shows dev is active
    let list = run_grit_command(&test_dir, &["branch"]).unwrap();
    assert!(list.contains("* dev"));
    assert!(list.contains("main"));

    // Add a file on dev branch
    fs::write(test_dir.path().join("dev.txt"), "dev content").unwrap();
    run_grit_command(&test_dir, &["add", "."]).unwrap();
    run_grit_command(&test_dir, &["commit", "-m", "Dev commit"]).unwrap();
    assert!(test_dir.path().join("dev.txt").exists());

    // Switch back to main
    let out = run_grit_command(&test_dir, &["checkout", "main"]).unwrap();
    assert!(out.contains("Switched to branch 'main'"));
    // dev.txt should NOT exist on main
    assert!(!test_dir.path().join("dev.txt").exists());
    assert!(test_dir.path().join("base.txt").exists());

    // Switch back to dev
    let out = run_grit_command(&test_dir, &["checkout", "dev"]).unwrap();
    assert!(out.contains("Switched to branch 'dev'"));
    // dev.txt should be restored
    assert!(test_dir.path().join("dev.txt").exists());
}
