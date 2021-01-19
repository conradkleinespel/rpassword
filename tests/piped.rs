//! This test checks that piped input is handled correctly.

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn piped_password() {
    // Run an example that reads a password and prints it
    let mut out = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--example")
        .arg("read-password")
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
