use crate::common::{run_grit_command, setup_integration_test};
use std::fs;

#[test]
fn test_commit_with_parent() {
    let test_dir = setup_integration_test();

    // Initialize and create first commit
    run_grit_command(&test_dir, &["init"]).unwrap();
    fs::write(test_dir.path().join("file1.txt"), "Content 1").unwrap();
    run_grit_command(&test_dir, &["commit", "--message", "First commit"]).unwrap();

    // Create second commit
    fs::write(test_dir.path().join("file2.txt"), "Content 2").unwrap();
    let commit_result = run_grit_command(&test_dir, &["commit", "--message", "Second commit"]);
    assert!(
        commit_result.is_ok(),
        "Second commit failed: {:?}",
        commit_result
    );

    // Verify HEAD points to new commit
    let head_content = fs::read_to_string(test_dir.path().join(".grit/HEAD")).unwrap();
    let head_ref = head_content.trim().strip_prefix("ref: ").unwrap();
    let branch_content = fs::read_to_string(test_dir.path().join(".grit").join(head_ref)).unwrap();
    let latest_commit = branch_content.trim();

    // Read the latest commit
    let cat_result = run_grit_command(&test_dir, &["cat-file", latest_commit]);
    assert!(cat_result.is_ok());
    let commit_content = cat_result.unwrap();
    assert!(commit_content.contains("parent"));
    assert!(commit_content.contains("Second commit"));
}

#[test]
fn test_log_command() {
    let test_dir = setup_integration_test();

    // Initialize and create commits
    run_grit_command(&test_dir, &["init"]).unwrap();
    fs::write(test_dir.path().join("file1.txt"), "Content 1").unwrap();
    run_grit_command(&test_dir, &["commit", "--message", "First commit"]).unwrap();

    fs::write(test_dir.path().join("file2.txt"), "Content 2").unwrap();
    run_grit_command(&test_dir, &["commit", "--message", "Second commit"]).unwrap();

    // Test log command
    let log_result = run_grit_command(&test_dir, &["log"]);
    assert!(log_result.is_ok(), "Log command failed: {:?}", log_result);
    let log_output = log_result.unwrap();
    assert!(log_output.contains("commit"));
    assert!(log_output.contains("Author:"));
    assert!(log_output.contains("Second commit"));
    assert!(log_output.contains("First commit"));

    // Test oneline format
    let oneline_result = run_grit_command(&test_dir, &["log", "--oneline"]);
    assert!(
        oneline_result.is_ok(),
        "Oneline log failed: {:?}",
        oneline_result
    );
    let oneline_output = oneline_result.unwrap();
    assert_eq!(oneline_output.lines().count(), 2); // Two commits
    assert!(oneline_output.contains("Second commit"));
    assert!(oneline_output.contains("First commit"));

    // Test -n 1 limit
    let limit_result = run_grit_command(&test_dir, &["log", "--oneline", "-n", "1"]).unwrap();
    assert_eq!(limit_result.lines().count(), 1);
    assert!(limit_result.contains("Second commit"));
    assert!(!limit_result.contains("First commit"));
}

#[test]
fn test_log_command_single_commit() {
    let test_dir = setup_integration_test();

    // Initialize and create one commit
    run_grit_command(&test_dir, &["init"]).unwrap();
    fs::write(test_dir.path().join("file.txt"), "Content").unwrap();
    run_grit_command(&test_dir, &["commit", "--message", "Single commit"]).unwrap();

    // Test log command
    let log_result = run_grit_command(&test_dir, &["log"]);
    assert!(log_result.is_ok());
    let log_output = log_result.unwrap();
    assert!(log_output.contains("commit"));
    assert!(log_output.contains("Author:"));
    assert!(log_output.contains("Single commit"));
    assert!(!log_output.contains("parent")); // No parent for first commit
}

#[test]
fn test_log_command_error_cases() {
    let test_dir = setup_integration_test();

    // Test log without repo
    let log_result = run_grit_command(&test_dir, &["log"]);
    assert!(log_result.is_err(), "Log should fail without commits");

    // Initialize but no commits
    run_grit_command(&test_dir, &["init"]).unwrap();
    let log_result = run_grit_command(&test_dir, &["log"]);
    assert!(log_result.is_err(), "Log should fail without commits");

    // Test invalid commit hash
    let log_result = run_grit_command(&test_dir, &["log", "invalidhash"]);
    assert!(log_result.is_err(), "Log should fail with invalid hash");
}
