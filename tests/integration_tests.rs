use std::process::Command;

#[test]
fn test_e2e_hello() {
    let output = Command::new("cargo")
        .args(&["run", "--release", "--", "run", "examples/hello.cez"])
        .output()
        .expect("Failed to run cez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from Cez!"));
    assert!(stdout.contains("Math check passed: 20 + 22 = 42"));
    assert!(stdout.contains("Deferred: execution completed cleanly!"));
}

#[test]
fn test_e2e_server_packet() {
    let output = Command::new("cargo")
        .args(&["run", "--release", "--", "run", "examples/server_packet.cez"])
        .output()
        .expect("Failed to run cez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[OK] Header validation passed!"));
    assert!(stdout.contains("payload_size=1024 bytes"));
}

#[test]
fn test_e2e_tui_dashboard() {
    let output = Command::new("cargo")
        .args(&["run", "--release", "--", "run", "examples/tui_dashboard.cez"])
        .output()
        .expect("Failed to run cez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CEZ SYSTEM SERVER DASHBOARD"));
    assert!(stdout.contains("HTTP Gateway"));
    assert!(stdout.contains("Queries: 14205"));
}

#[test]
fn test_e2e_freestanding_kernel() {
    let temp_o = std::env::temp_dir().join("test_kernel.o");
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--",
            "build",
            "examples/kernel_multiboot.cez",
            "--freestanding",
            "-o",
            temp_o.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run cez");

    assert!(output.status.success());
    assert!(temp_o.exists());

    // Check ELF section .multiboot using readelf
    let readelf_out = Command::new("readelf")
        .args(&["-S", temp_o.to_str().unwrap()])
        .output()
        .expect("Failed to run readelf");

    let stdout = String::from_utf8_lossy(&readelf_out.stdout);
    assert!(stdout.contains(".multiboot"));
    assert!(stdout.contains(".bss"));

    let _ = std::fs::remove_file(temp_o);
}
