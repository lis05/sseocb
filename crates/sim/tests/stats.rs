use std::process::Command;

#[test]
fn test_sim_run_stats() {
    let sim_bin = env!("CARGO_BIN_EXE_sim");
    let prime_bin = concat!(env!("CARGO_MANIFEST_DIR"), "/test_run/primes.bin");

    let output = Command::new(sim_bin)
        .args(["run", "--stats", prime_bin])
        .output()
        .expect("failed to execute sim run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Cycles executed before ebreak: "),
        "expected stats in stdout, got:\n{}",
        stdout
    );
}

#[test]
fn test_sim_debug_stats() {
    use std::io::Write;
    use std::process::Stdio;

    let sim_bin = env!("CARGO_BIN_EXE_sim");
    let prime_bin = concat!(env!("CARGO_MANIFEST_DIR"), "/test_run/primes.bin");

    let mut child = Command::new(sim_bin)
        .args(["debug", "--stats", prime_bin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn sim debug");

    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "c").unwrap();
        writeln!(stdin, "end").unwrap();
    }

    let output = child.wait_with_output().expect("failed to read output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Cycles executed before ebreak: "),
        "expected stats in debug stdout, got:\n{}",
        stdout
    );
}
