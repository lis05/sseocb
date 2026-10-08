use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_sim_accept_all_fixtures() {
    let sim_bin = env!("CARGO_BIN_EXE_sim");
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");

    let entries = fs::read_dir(&fixtures_dir).unwrap_or_else(|e| {
        panic!(
            "failed to read fixtures directory {:?}: {}",
            fixtures_dir, e
        )
    });

    let mut tested_count = 0;
    for entry in entries {
        let entry = entry.expect("valid directory entry");
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("bin") {
            let output = Command::new(sim_bin)
                .args(["accept", path.to_str().unwrap()])
                .output()
                .unwrap_or_else(|e| panic!("failed to execute sim on {:?}: {}", path, e));

            assert!(
                output.status.success(),
                "sim accept failed for fixture {:?}:\nStdout: {}\nStderr: {}",
                path.file_name().unwrap(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );

            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains("Accepted: successfully decoded"),
                "unexpected output for {:?}: {}",
                path.file_name().unwrap(),
                stdout
            );

            tested_count += 1;
        }
    }

    assert!(
        tested_count > 0,
        "no .bin fixtures found in {:?}",
        fixtures_dir
    );
}

#[test]
fn test_sim_accept_rejects_corrupted_file() {
    let sim_bin = env!("CARGO_BIN_EXE_sim");
    let temp_dir = std::env::temp_dir();
    let bad_bin = temp_dir.join(format!("corrupted_rv32i_{}.bin", std::process::id()));
    std::fs::write(&bad_bin, [0u8; 4]).unwrap();

    let output = Command::new(sim_bin)
        .args(["accept", bad_bin.to_str().unwrap()])
        .output()
        .expect("failed to execute sim binary");

    let _ = std::fs::remove_file(bad_bin);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("rejected instruction at byte offset 0x00000000: 0x00000000"));
}

#[test]
fn test_sim_accept_rejects_non_multiple_of_4() {
    let sim_bin = env!("CARGO_BIN_EXE_sim");
    let temp_dir = std::env::temp_dir();
    let odd_bin = temp_dir.join(format!("odd_length_{}.bin", std::process::id()));
    std::fs::write(&odd_bin, [0x13, 0x00, 0x00]).unwrap();

    let output = Command::new(sim_bin)
        .args(["accept", odd_bin.to_str().unwrap()])
        .output()
        .expect("failed to execute sim binary");

    let _ = std::fs::remove_file(odd_bin);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("is not a multiple of 4 bytes"));
}
