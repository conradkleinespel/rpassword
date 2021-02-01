//! This example demonstrates how to read a password from the tty.

extern crate rpassword;

fn main() {
    println!("Prompt:");
    let pass = rpassword::read_password_from_tty().unwrap();
    println!("Password: {}", pass);
}
