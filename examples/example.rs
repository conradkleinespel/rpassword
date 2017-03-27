extern crate rpassword;

use std::io::{stdout, Write};

fn main() {
    let mut stdout = stdout();

    print!("Password: ");
    stdout.flush().unwrap();
    let pass = rpassword::read_password().unwrap();
    println!("Your password is {}", pass);

    let pass = rpassword::prompt_password_stdout("Password with prompt on stdout: ").unwrap();
    println!("Your password is {}", pass);

    let pass = rpassword::prompt_password_stderr("Password with prompt on stderr: ").unwrap();
    println!("Your password is {}", pass);

    let response = rpassword::read_response().unwrap();
    println!("Your response is {}", response);

    let response = rpassword::prompt_response_stdout("Response with prompt on stdout: ").unwrap();
    println!("Your response is {}", response);

    let response = rpassword::prompt_response_stderr("Response with prompt on stderr: ").unwrap();
    println!("Your response is {}", response);
}
