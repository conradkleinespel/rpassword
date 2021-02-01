//! This example demonstrates how to read a password from STDIN.

extern crate rpassword;

use std::io::stdin;

fn main() {
    println!("Prompt:");
    let pass = rpassword::read_password_from_stdin_lock(&mut stdin().lock()).unwrap();
    println!("Password: {}", pass);
}
