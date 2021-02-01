//! This example demonstrates how to read a password from STDIN.

extern crate rpassword;

use std::io::Cursor;

fn main() {
    println!("Prompt:");
    let mut cursor = Cursor::new("Ann Onymous".as_bytes().to_vec());
    let pass = rpassword::read_password_from_bufread(&mut cursor).unwrap();
    println!("Password: {}", pass);
}
