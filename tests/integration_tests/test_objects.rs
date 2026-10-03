use crate::common::{run_grit_command, setup_integration_test};
use std::fs;
use std::process::Command;

#[test]
fn test_cat_file_tree() {
    let test_dir = setup_integration_test();

    run_grit_command(&test_dir, &["init"]).unwrap();
    fs::write(test_dir.path().join("hello.txt"), "Hello, World!").unwrap();
    run_grit_command(&test_dir, &["add", "hello.txt"]).unwrap();
    let tree_hash = run_grit_command(&test_dir, &["write-tree"]).unwrap();

    let cat_result = run_grit_command(&test_dir, &["cat-file", &tree_hash]).unwrap();
    assert!(cat_result.contains("hello.txt"));
    assert!(cat_result.contains("100644"));
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
