use crate::common::{run_grit_command, setup_integration_test};
use std::fs;

#[test]
fn test_diff_workflow() {
    let test_dir = setup_integration_test();

    run_grit_command(&test_dir, &["init"]).unwrap();

    // Create and stage initial file
    fs::write(test_dir.path().join("file.txt"), "line 1\nline 2\n").unwrap();
    run_grit_command(&test_dir, &["add", "file.txt"]).unwrap();

    // Staged diff before any commit (all added)
    let staged_diff = run_grit_command(&test_dir, &["diff", "--staged"]).unwrap();
    assert!(staged_diff.contains("new file mode"));
    assert!(staged_diff.contains("+line 1"));
    assert!(staged_diff.contains("+line 2"));

    // Staged diff with --cached alias
    let cached_diff = run_grit_command(&test_dir, &["diff", "--cached"]).unwrap();
    assert_eq!(staged_diff, cached_diff);

    // Initial commit
    let commit1 = run_grit_command(&test_dir, &["commit", "-m", "Initial commit"]).unwrap();

    // Working tree is clean, diff should be empty
    let diff_clean = run_grit_command(&test_dir, &["diff"]).unwrap();
    assert!(diff_clean.is_empty());

    // Modify file in working tree (unstaged)
    fs::write(test_dir.path().join("file.txt"), "line 1\nline 2 mod\n").unwrap();
    let diff_unstaged = run_grit_command(&test_dir, &["diff"]).unwrap();
    assert!(diff_unstaged.contains("--- a/file.txt"));
    assert!(diff_unstaged.contains("+++ b/file.txt"));
    assert!(diff_unstaged.contains("-line 2"));
    assert!(diff_unstaged.contains("+line 2 mod"));

    // Test diff --stat
    let diff_stat = run_grit_command(&test_dir, &["diff", "--stat"]).unwrap();
    assert!(diff_stat.contains("file.txt"));
    assert!(diff_stat.contains("1 file changed, 1 insertion(+), 1 deletion(-)"));

    // Stage modification
    run_grit_command(&test_dir, &["add", "file.txt"]).unwrap();
    let diff_after_add = run_grit_command(&test_dir, &["diff"]).unwrap();
    assert!(diff_after_add.is_empty());

    let staged_diff2 = run_grit_command(&test_dir, &["diff", "--staged"]).unwrap();
    assert!(staged_diff2.contains("-line 2"));
    assert!(staged_diff2.contains("+line 2 mod"));

    // Second commit
    let commit2 = run_grit_command(&test_dir, &["commit", "-m", "Second commit"]).unwrap();

    // Diff between commit1 and commit2
    let commit_diff = run_grit_command(&test_dir, &["diff", &commit1, &commit2]).unwrap();
    assert!(commit_diff.contains("-line 2"));
    assert!(commit_diff.contains("+line 2 mod"));

    // Diff HEAD vs working tree after further change
    fs::write(
        test_dir.path().join("file.txt"),
        "line 1\nline 2 mod\nline 3\n",
    )
    .unwrap();
    let diff_head = run_grit_command(&test_dir, &["diff", "HEAD"]).unwrap();
    assert!(diff_head.contains("+line 3"));

    // Working tree file deletion diff
    fs::remove_file(test_dir.path().join("file.txt")).unwrap();
    let diff_del = run_grit_command(&test_dir, &["diff"]).unwrap();
    assert!(diff_del.contains("deleted file mode"));
    assert!(diff_del.contains("--- a/file.txt"));
    assert!(diff_del.contains("+++ /dev/null"));

    // Working tree vs earlier commit (commit1)
    fs::write(test_dir.path().join("file.txt"), "line 1\nline 2 new\n").unwrap();
    let diff_commit1 = run_grit_command(&test_dir, &["diff", &commit1]).unwrap();
    assert!(diff_commit1.contains("-line 2"));
    assert!(diff_commit1.contains("+line 2 new"));

    // Error case: --staged with two commits
    let err = run_grit_command(&test_dir, &["diff", "--staged", &commit1, &commit2]);
    assert!(err.is_err());
    assert!(
        err.unwrap_err()
            .contains("Cannot use --staged with two commits")
    );
}
