
#![feature(io)]

extern crate rpassword;

use std::old_io::stdio::{flush, stdin};

fn main() {
    print!("Password: ");
    flush();
    let pass = rpassword::read_password().unwrap();
    println!("Your password is {:?}", pass);
    let mut stdin = stdin();
    print!("Plaintext: ");
    flush();
    let line = stdin.read_line();
    println!("Your plaintext is {:?}", line);
}
