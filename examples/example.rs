extern crate rpassword;

use std::io::{stdout, Write, stdin, BufRead};

fn main() {
    let mut stdout = stdout();

    print!("Password: ");
    stdout.flush().unwrap();
    let pass = rpassword::read_password().unwrap();
    println!("Your password is {:?}", pass);

    let mut plaintext = String::new();
    let stdin = stdin();
    print!("Plaintext: ");
    stdout.flush().unwrap();
    stdin.lock().read_line(&mut plaintext).unwrap();
    println!("Your plaintext is {:?}", plaintext);
}
