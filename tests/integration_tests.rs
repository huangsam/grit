use assert_cmd::cargo::cargo_bin;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn setup_integration_test() -> TempDir {
    TempDir::new().unwrap()
}

fn run_grit_command(test_dir: &TempDir, args: &[&str]) -> Result<String, String> {
    let grit_binary = cargo_bin("grit");

    println!(
        "Running: {} {:?} in {:?}",
        grit_binary.display(),
        args,
        test_dir
    );

    let output = Command::new(&grit_binary)
        .args(args)
        .current_dir(test_dir)
        .output()
        .map_err(|e| format!("Failed to run command: {}", e))?;

    if output.status.success() {
        // For cat-file command, don't trim to preserve exact content
        if args.first().copied() == Some("cat-file") {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("stdout: {}\nstderr: {}", stdout, stderr))
    }
}

#[test]
fn test_full_git_workflow() {
    let test_dir = setup_integration_test();

    // Initialize repository
    let result = run_grit_command(&test_dir, &["init"]);
    assert!(result.is_ok(), "Init failed: {:?}", result);

    // Verify .grit directory exists
    assert!(test_dir.path().join(".grit").exists());
    assert!(test_dir.path().join(".grit/objects").exists());
    assert!(test_dir.path().join(".grit/refs").exists());

    // Create a test file
    fs::write(test_dir.path().join("hello.txt"), "Hello, World!").unwrap();

    // Hash the file
    let hash_result = run_grit_command(&test_dir, &["hash-object", "hello.txt"]);
    assert!(hash_result.is_ok(), "Hash-object failed: {:?}", hash_result);
    let blob_hash = hash_result.unwrap();
    assert_eq!(blob_hash.len(), 40);

    // Create tree snapshot
    let tree_result = run_grit_command(&test_dir, &["write-tree"]);
    assert!(tree_result.is_ok(), "Write-tree failed: {:?}", tree_result);
    let tree_hash = tree_result.unwrap();
    assert_eq!(tree_hash.len(), 40);

    // Create commit
    let commit_result = run_grit_command(&test_dir, &["commit", "--message", "Initial commit"]);
    assert!(commit_result.is_ok(), "Commit failed: {:?}", commit_result);
    let commit_hash = commit_result.unwrap();
    assert_eq!(commit_hash.len(), 40);

    // Verify commit object exists
    let commit_file = test_dir
        .path()
        .join(".grit/objects")
        .join(&commit_hash[..2])
        .join(&commit_hash[2..]);
    assert!(commit_file.exists(), "Commit object file should exist");

    // Read the commit
    let cat_result = run_grit_command(&test_dir, &["cat-file", &commit_hash]);
    assert!(cat_result.is_ok(), "Cat-file failed: {:?}", cat_result);
    let commit_content = cat_result.unwrap();
    assert!(commit_content.contains("tree"));
    assert!(commit_content.contains("author"));
    assert!(commit_content.contains("committer"));
    assert!(commit_content.contains("Initial commit"));
}

#[test]
fn test_error_cases() {
    let test_dir = setup_integration_test();

    // Try to initialize twice
    let _ = run_grit_command(&test_dir, &["init"]);
    let double_init = run_grit_command(&test_dir, &["init"]);
    assert!(double_init.is_err(), "Double init should fail");

    // Try to read nonexistent object
    let nonexistent = run_grit_command(
        &test_dir,
        &["cat-file", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
    );
    assert!(
        nonexistent.is_err(),
        "Reading nonexistent object should fail"
    );
}

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
    assert!(oneline_output.lines().count() == 2); // Two commits
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
fn test_git_compatibility() {
    let test_dir = setup_integration_test();

    // Initialize grit repository
    run_grit_command(&test_dir, &["init"]).unwrap();

    // Create test files with various content types
    let test_files = vec![
        ("empty.txt", ""),
        ("simple.txt", "Hello World"),
        ("unicode.txt", "🚀 Hello 世界 🌍"),
        ("multiline.txt", "Line 1\nLine 2\nLine 3\n"),
    ];

    for (filename, content) in test_files {
        fs::write(test_dir.path().join(filename), content).unwrap();

        // Get grit hash
        let grit_hash = run_grit_command(&test_dir, &["hash-object", filename]).unwrap();

        // Get git hash for comparison
        let git_output = Command::new("git")
            .args(["hash-object", filename])
            .current_dir(test_dir.path())
            .output()
            .expect("git command failed");

        assert!(git_output.status.success(), "git hash-object failed");
        let git_hash = String::from_utf8_lossy(&git_output.stdout)
            .trim()
            .to_string();

        // Compare hashes
        assert_eq!(
            grit_hash, git_hash,
            "Hash mismatch for file {}: grit={}, git={}",
            filename, grit_hash, git_hash
        );

        // Verify we can read the object back
        let cat_result = run_grit_command(&test_dir, &["cat-file", &grit_hash]);
        assert!(cat_result.is_ok(), "Failed to read object {}", grit_hash);
        let read_content = cat_result.unwrap();

        // Compare content
        assert_eq!(read_content, content);
    }

    // Test binary file separately
    let binary_content = vec![0, 1, 255, 128];
    fs::write(test_dir.path().join("binary.dat"), &binary_content).unwrap();

    let grit_hash = run_grit_command(&test_dir, &["hash-object", "binary.dat"]).unwrap();

    let git_output = Command::new("git")
        .args(["hash-object", "binary.dat"])
        .current_dir(test_dir.path())
        .output()
        .expect("git command failed");

    assert!(
        git_output.status.success(),
        "git hash-object failed for binary file"
    );
    let git_hash = String::from_utf8_lossy(&git_output.stdout)
        .trim()
        .to_string();

    assert_eq!(
        grit_hash, git_hash,
        "Hash mismatch for binary file: grit={}, git={}",
        grit_hash, git_hash
    );

    // For binary content, compare by reading the object directly instead of through cat-file
    let read_obj = grit::plumbing::objects::read_object(&grit_hash, test_dir.path()).unwrap();
    assert_eq!(read_obj.obj_type, grit::plumbing::objects::ObjectType::Blob);
    assert_eq!(read_obj.content, binary_content);
}

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

    // Error case: --staged with two commits
    let err = run_grit_command(&test_dir, &["diff", "--staged", &commit1, &commit2]);
    assert!(err.is_err());
    assert!(
        err.unwrap_err()
            .contains("Cannot use --staged with two commits")
    );
}

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
    assert!(reset_default.is_ok(), "Reset default failed: {:?}", reset_default);

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
    assert!(reset_mixed.is_ok(), "Reset --mixed failed: {:?}", reset_mixed);

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

#[test]
fn test_gritignore_workflow() {
    let test_dir = setup_integration_test();

    run_grit_command(&test_dir, &["init"]).unwrap();

    // Create .gritignore file
    let ignore_content = "*.tmp\nbuild/\nsecret.txt\n";
    fs::write(test_dir.path().join(".gritignore"), ignore_content).unwrap();

    // Create tracked and ignored files
    fs::write(test_dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(test_dir.path().join("test.tmp"), "temporary data\n").unwrap();
    fs::write(test_dir.path().join("secret.txt"), "secret data\n").unwrap();
    fs::create_dir(test_dir.path().join("build")).unwrap();
    fs::write(
        test_dir.path().join("build").join("bundle.js"),
        "console.log();\n",
    )
    .unwrap();

    // Status before add: main.rs and .gritignore should show as untracked
    // Ignored files (test.tmp, secret.txt, build/bundle.js) must NOT appear
    let status_before_add = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_before_add.contains("Untracked files:"));
    assert!(status_before_add.contains("main.rs"));
    assert!(status_before_add.contains(".gritignore"));
    assert!(!status_before_add.contains("test.tmp"));
    assert!(!status_before_add.contains("secret.txt"));
    assert!(!status_before_add.contains("bundle.js"));
    assert!(!status_before_add.contains("build"));

    // Run grit add . (stages main.rs, skips .gritignore and ignored files)
    let add_result = run_grit_command(&test_dir, &["add", "."]);
    assert!(add_result.is_ok(), "Add failed: {:?}", add_result);

    // Status after add: main.rs is staged, ignored files are not staged
    let status_after_add = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_after_add.contains("Changes to be committed:"));
    assert!(status_after_add.contains("new file: main.rs"));
    assert!(!status_after_add.contains("test.tmp"));
    assert!(!status_after_add.contains("secret.txt"));
    assert!(!status_after_add.contains("bundle.js"));

    // Stage .gritignore explicitly and commit both
    run_grit_command(&test_dir, &["add", ".gritignore"]).unwrap();
    run_grit_command(&test_dir, &["commit", "-m", "Initial commit"]).unwrap();

    // After commit, working tree should report clean despite ignored files existing
    let status_after_commit = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_after_commit.contains("nothing to commit, working tree clean"));

    // Adding an ignored file pattern with grit add *.tmp should not add test.tmp
    let add_glob = run_grit_command(&test_dir, &["add", "*.tmp"]);
    assert!(add_glob.is_ok());
    let status_after_glob = run_grit_command(&test_dir, &["status"]).unwrap();
    assert!(status_after_glob.contains("nothing to commit, working tree clean"));
}


