extern crate rpassword;

use std::io::{stdout, Write};

fn main() {
    let mut stdout = stdout();

    print!("Password: ");
    stdout.flush().unwrap();
    let pass = rpassword::read_password().unwrap();
    println!("Your password is {}", pass);

    let pass = rpassword::prompt_password_stdout("Password: ").unwrap();
    println!("Your password is {}", pass);

    let pass = rpassword::prompt_password_stderr("Password: ").unwrap();
    println!("Your password is {}", pass);
}
