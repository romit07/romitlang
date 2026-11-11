use std::process::Command;

#[test]
fn version_flag_matches_cargo_toml() {
    let exe = env!("CARGO_BIN_EXE_romit");
    let expected = env!("CARGO_PKG_VERSION");

    for flag in ["--version", "-V"] {
        let output = Command::new(exe)
            .arg(flag)
            .output()
            .expect("failed to run romit");
        assert!(
            output.status.success(),
            "romit {} failed: {}",
            flag,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
        assert!(
            stdout.trim().contains(expected),
            "expected {} in output {:?}",
            expected,
            stdout
        );
    }
}
