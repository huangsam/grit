use crate::common::{run_grit_command, setup_integration_test};
use std::fs;

#[test]
fn test_branch_workflow() {
    let test_dir = setup_integration_test();

    run_grit_command(&test_dir, &["init"]).unwrap();
    fs::write(test_dir.path().join("file.txt"), "hello").unwrap();
    run_grit_command(&test_dir, &["add", "."]).unwrap();
    run_grit_command(&test_dir, &["commit", "-m", "First commit"]).unwrap();

    // Verify initial branch list
    let list_output = run_grit_command(&test_dir, &["branch"]).unwrap();
    assert!(list_output.contains("* main"));

    // Create a new branch
    let create_output = run_grit_command(&test_dir, &["branch", "feature"]).unwrap();
    assert!(create_output.is_empty() || !create_output.contains("Error"));

    // List branches - should show both with main active
    let list_output2 = run_grit_command(&test_dir, &["branch"]).unwrap();
    assert!(list_output2.contains("* main"));
    assert!(list_output2.contains("feature"));

    // Delete the feature branch
    let delete_output = run_grit_command(&test_dir, &["branch", "-d", "feature"]).unwrap();
    assert!(delete_output.contains("Deleted branch feature"));

    // List branches - feature should be gone
    let list_output3 = run_grit_command(&test_dir, &["branch"]).unwrap();
    assert!(list_output3.contains("* main"));
    assert!(!list_output3.contains("feature"));
}

#[test]
fn test_branch_error_cases() {
    let test_dir = setup_integration_test();

    run_grit_command(&test_dir, &["init"]).unwrap();

    // 1. Cannot create branch before any commits
    let err_no_commit = run_grit_command(&test_dir, &["branch", "feature"]);
    assert!(err_no_commit.is_err());
    assert!(
        err_no_commit
            .unwrap_err()
            .contains("not a valid object name: 'HEAD'")
    );

    // Create initial commit
    fs::write(test_dir.path().join("file.txt"), "hello").unwrap();
    run_grit_command(&test_dir, &["add", "."]).unwrap();
    run_grit_command(&test_dir, &["commit", "-m", "Initial"]).unwrap();

    // 2. Cannot create branch that already exists
    let err_exists = run_grit_command(&test_dir, &["branch", "main"]);
    assert!(err_exists.is_err());
    assert!(err_exists.unwrap_err().contains("already exists"));

    // 3. Cannot delete currently checked out branch
    let err_delete_current = run_grit_command(&test_dir, &["branch", "-d", "main"]);
    assert!(err_delete_current.is_err());
    assert!(err_delete_current.unwrap_err().contains("checked out"));

    // 4. Cannot delete nonexistent branch
    let err_not_found = run_grit_command(&test_dir, &["branch", "-d", "nonexistent"]);
    assert!(err_not_found.is_err());
    assert!(err_not_found.unwrap_err().contains("not found"));

    // 5. Cannot create branch with invalid ref name
    let err_invalid_name = run_grit_command(&test_dir, &["branch", "invalid..name"]);
    assert!(err_invalid_name.is_err());
}
