//! This test checks that piped input is handled correctly.

use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn piped_password() {
    // Find target directory
    let target_dir = env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    // Run an example that reads a password and prints it
    let mut out = Command::new(target_dir.join("examples/read-password"))
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    
    // Write "secret" as the password into stdin
    let stdin = out.stdin.as_mut().unwrap();
    stdin.write_all("secret".as_bytes()).unwrap();

    let out = out.wait_with_output().unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    println!("stdout: {}", stdout);
    assert!(stdout.ends_with("Password: secret\n"));
}
