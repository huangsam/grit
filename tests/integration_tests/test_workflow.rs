use crate::common::{run_grit_command, setup_integration_test};
use std::fs;

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
