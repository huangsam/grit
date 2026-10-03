use crate::common::{run_grit_command, setup_integration_test};
use std::fs;

#[test]
fn test_reset_workflow() {
    let test_dir = setup_integration_test();

    run_grit_command(&test_dir, &["init"]).unwrap();

    // Commit 1: file1.txt
    fs::write(test_dir.path().join("file1.txt"), "v1\n").unwrap();
    run_grit_command(&test_dir, &["add", "file1.txt"]).unwrap();
    let commit1 = run_grit_command(&test_dir, &["commit", "-m", "Commit 1"]).unwrap();

    // Commit 2: file2.txt, modify file1.txt
    fs::write(test_dir.path().join("file1.txt"), "v2\n").unwrap();
    fs::write(test_dir.path().join("file2.txt"), "v2-file2\n").unwrap();
    run_grit_command(&test_dir, &["add", "."]).unwrap();
    let _commit2 = run_grit_command(&test_dir, &["commit", "-m", "Commit 2"]).unwrap();

    // 1. Test --soft reset to commit1
    // HEAD should move to commit1, but index and working tree keep changes from commit2
    let reset_soft = run_grit_command(&test_dir, &["reset", "--soft", &commit1]);
    assert!(reset_soft.is_ok(), "Reset --soft failed: {:?}", reset_soft);

    // HEAD points to commit1
    let log_out = run_grit_command(&test_dir, &["log", "--oneline"]).unwrap();
    assert_eq!(log_out.lines().count(), 1);
    assert!(log_out.contains("Commit 1"));

    // Working directory unchanged
    assert_eq!(
        fs::read_to_string(test_dir.path().join("file1.txt")).unwrap(),
        "v2\n"
    );
    assert_eq!(
        fs::read_to_string(test_dir.path().join("file2.txt")).unwrap(),
        "v2-file2\n"
    );

    // Status shows staged changes
    let status_after_soft = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_after_soft.contains("Changes to be committed:"));
    assert!(status_after_soft.contains("modified: file1.txt"));
    assert!(status_after_soft.contains("new file: file2.txt"));

    // 2. Test default (mixed) reset with no commit argument (resets staged changes against HEAD)
    let reset_default = run_grit_command(&test_dir, &["reset"]);
    assert!(
        reset_default.is_ok(),
        "Reset default failed: {:?}",
        reset_default
    );

    let status_after_default = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(!status_after_default.contains("Changes to be committed:"));
    assert!(status_after_default.contains("Changes not staged for commit:"));
    assert!(status_after_default.contains("modified: file1.txt"));
    assert!(status_after_default.contains("Untracked files:"));
    assert!(status_after_default.contains("file2.txt"));

    // Re-commit to advance to commit2 again
    run_grit_command(&test_dir, &["add", "."]).unwrap();
    let _commit2_again = run_grit_command(&test_dir, &["commit", "-m", "Commit 2 again"]).unwrap();

    // 3. Test --mixed reset to commit1
    let reset_mixed = run_grit_command(&test_dir, &["reset", "--mixed", &commit1]);
    assert!(
        reset_mixed.is_ok(),
        "Reset --mixed failed: {:?}",
        reset_mixed
    );

    // HEAD moved to commit1
    let log_out = run_grit_command(&test_dir, &["log", "--oneline"]).unwrap();
    assert_eq!(log_out.lines().count(), 1);
    assert!(log_out.contains("Commit 1"));

    // Working tree still has v2 changes, but index is reset (unstaged)
    let status_after_mixed = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(!status_after_mixed.contains("Changes to be committed:"));
    assert!(status_after_mixed.contains("modified: file1.txt"));
    assert!(status_after_mixed.contains("Untracked files:"));
    assert!(status_after_mixed.contains("file2.txt"));

    // Re-commit commit2
    run_grit_command(&test_dir, &["add", "."]).unwrap();
    let _commit2_third = run_grit_command(&test_dir, &["commit", "-m", "Commit 2 third"]).unwrap();

    // 4. Test --hard reset to commit1
    // Working tree should be restored to commit1 state, file2.txt deleted, file1.txt reverted
    let reset_hard = run_grit_command(&test_dir, &["reset", "--hard", &commit1]);
    assert!(reset_hard.is_ok(), "Reset --hard failed: {:?}", reset_hard);

    // file1.txt is v1
    assert_eq!(
        fs::read_to_string(test_dir.path().join("file1.txt")).unwrap(),
        "v1\n"
    );
    // file2.txt is deleted
    assert!(!test_dir.path().join("file2.txt").exists());

    // Status is completely clean
    let status_after_hard = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_after_hard.contains("nothing to commit, working tree clean"));

    // 5. Test path-based reset: `grit reset -- <paths>`
    // Create new files and stage them
    fs::write(test_dir.path().join("file1.txt"), "v1-modified\n").unwrap();
    fs::write(test_dir.path().join("staged_new.txt"), "new file\n").unwrap();
    run_grit_command(&test_dir, &["add", "."]).unwrap();

    // Unstage only file1.txt
    let reset_path = run_grit_command(&test_dir, &["reset", "--", "file1.txt"]);
    assert!(reset_path.is_ok(), "Reset path failed: {:?}", reset_path);

    // file1.txt should be unstaged (in 'Changes not staged for commit')
    // staged_new.txt should still be staged (in 'Changes to be committed')
    let status_after_path = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_after_path.contains("Changes to be committed:"));
    assert!(status_after_path.contains("new file: staged_new.txt"));
    assert!(status_after_path.contains("Changes not staged for commit:"));
    assert!(status_after_path.contains("modified: file1.txt"));
}

#[test]
fn test_reset_error_cases() {
    let test_dir = setup_integration_test();

    // Reset without initialized repo
    let err_no_repo = run_grit_command(&test_dir, &["reset"]);
    assert!(err_no_repo.is_err());

    // Reset on empty repo without commits
    run_grit_command(&test_dir, &["init"]).unwrap();
    let err_no_commits = run_grit_command(&test_dir, &["reset"]);
    assert!(err_no_commits.is_err());
    assert!(err_no_commits.unwrap_err().contains("NoCommits"));

    // Commit one file
    fs::write(test_dir.path().join("file.txt"), "hello").unwrap();
    run_grit_command(&test_dir, &["add", "file.txt"]).unwrap();
    run_grit_command(&test_dir, &["commit", "-m", "First"]).unwrap();

    // Reset with --hard and paths
    let err_hard_paths = run_grit_command(&test_dir, &["reset", "--hard", "--", "file.txt"]);
    assert!(err_hard_paths.is_err());
    assert!(
        err_hard_paths
            .unwrap_err()
            .contains("Cannot use --hard or --soft with paths")
    );

    // Reset with --soft and paths
    let err_soft_paths = run_grit_command(&test_dir, &["reset", "--soft", "--", "file.txt"]);
    assert!(err_soft_paths.is_err());
    assert!(
        err_soft_paths
            .unwrap_err()
            .contains("Cannot use --hard or --soft with paths")
    );

    // Reset to nonexistent commit
    let err_invalid_commit = run_grit_command(
        &test_dir,
        &["reset", "0000000000000000000000000000000000000000"],
    );
    assert!(err_invalid_commit.is_err());
}
