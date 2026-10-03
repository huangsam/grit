use assert_cmd::cargo::cargo_bin;
use std::process::Command;
use tempfile::TempDir;

pub fn setup_integration_test() -> TempDir {
    TempDir::new().unwrap()
}

pub fn run_grit_command(test_dir: &TempDir, args: &[&str]) -> Result<String, String> {
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
