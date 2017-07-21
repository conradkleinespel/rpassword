extern crate rpassword;

use std::io::{stdout, Write};

fn main() {
    let mut stdout = stdout();

    // Password without prompt
    let pass = rpassword::read_password().unwrap();
    println!("Your password is {}", pass);

    let pass = rpassword::prompt_password_stdout("Password with prompt on stdout: ").unwrap();
    println!("Your password is {}", pass);

    let pass = rpassword::prompt_password_stderr("Password with prompt on stderr: ").unwrap();
    println!("Your password is {}", pass);

    // Password (displayed, not hidden) without prompt
    let pass = rpassword::read_response().unwrap();
    println!("Your password is {}", pass);

    let pass = rpassword::prompt_response_stdout("Password (displayed, not hidden) with prompt on stdout: ").unwrap();
    println!("Your password is {}", pass);

    let pass = rpassword::prompt_response_stderr("Password (displayed, not hidden) with prompt on stderr: ").unwrap();
    println!("Your password is {}", pass);
}
