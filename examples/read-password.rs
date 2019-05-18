//! This example demonstrates how to read a password from STDIN.

extern crate rpassword;

fn main() {
    let pass = rpassword::read_password().unwrap();
    println!("Password: {}", pass);
}
