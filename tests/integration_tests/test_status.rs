use crate::common::{run_grit_command, setup_integration_test};
use std::fs;

#[test]
fn test_status_workflow() {
    let test_dir = setup_integration_test();

    run_grit_command(&test_dir, &["init"]).unwrap();
    let status_init = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_init.contains("On branch main"));
    assert!(status_init.contains("No commits yet"));

    // Add a file and check status
    fs::write(test_dir.path().join("file.txt"), "content").unwrap();
    let status_untracked = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_untracked.contains("On branch main"));
    assert!(status_untracked.contains("Untracked files:"));
    assert!(status_untracked.contains("file.txt"));

    // Stage the file and check status
    run_grit_command(&test_dir, &["add", "file.txt"]).unwrap();
    let status_staged = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_staged.contains("Changes to be committed:"));
    assert!(status_staged.contains("new file: file.txt"));

    // Commit and verify clean status
    let commit1 = run_grit_command(&test_dir, &["commit", "-m", "Commit"]).unwrap();
    let status_clean = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_clean.contains("On branch main"));
    assert!(status_clean.contains("nothing to commit, working tree clean"));

    // 1. Unstaged modification
    fs::write(test_dir.path().join("file.txt"), "modified content").unwrap();
    let status_unstaged_mod = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_unstaged_mod.contains("Changes not staged for commit:"));
    assert!(status_unstaged_mod.contains("modified: file.txt"));
    assert!(!status_unstaged_mod.contains("Changes to be committed:"));

    // 2. Staged modification (shows as 'modified: file.txt' instead of 'new file: file.txt')
    run_grit_command(&test_dir, &["add", "file.txt"]).unwrap();
    let status_staged_mod = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_staged_mod.contains("Changes to be committed:"));
    assert!(status_staged_mod.contains("modified: file.txt"));
    assert!(!status_staged_mod.contains("new file: file.txt"));

    let commit2 = run_grit_command(&test_dir, &["commit", "-m", "Commit 2"]).unwrap();

    // 3. Unstaged deletion
    fs::remove_file(test_dir.path().join("file.txt")).unwrap();
    let status_deleted = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_deleted.contains("Changes not staged for commit:"));
    assert!(status_deleted.contains("deleted: file.txt"));

    // Restore file and checkout detached HEAD
    run_grit_command(&test_dir, &["checkout", &commit2]).unwrap();
    let status_detached = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_detached.contains(&format!("HEAD detached at {}", &commit2[..7])));
    assert!(status_detached.contains("nothing to commit, working tree clean"));

    // Switch back to commit1 directly (detached HEAD on past commit)
    run_grit_command(&test_dir, &["checkout", &commit1]).unwrap();
    let status_detached_commit1 = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_detached_commit1.contains(&format!("HEAD detached at {}", &commit1[..7])));
}
