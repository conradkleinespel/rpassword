//! This example demonstrates how to read a password from STDIN.

fn main() {
    println!("Prompt:");
    let pass = rpassword::read_password().unwrap();
    println!("Password: {}", pass);
}
