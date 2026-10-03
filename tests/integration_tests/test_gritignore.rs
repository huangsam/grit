use crate::common::{run_grit_command, setup_integration_test};
use std::fs;

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
