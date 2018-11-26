//! This example demonstrates how to read a password from the tty.

extern crate rpassword;

fn main() {
    let pass = rpassword::read_password_from_tty(Some("Password: ")).unwrap();
    println!("Password: {}", pass);
}
