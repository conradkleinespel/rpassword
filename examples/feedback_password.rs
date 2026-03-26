use rpassword::PasswordFeedback;

fn main() {
    println!("=== Hide mode (default behavior) ===");
    match rpassword::prompt_password_with_feedback("Password: ", PasswordFeedback::Hide) {
        Ok(pass) => println!("You entered: {}", pass),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\n=== Mask('*') mode ===");
    match rpassword::prompt_password_with_feedback("Password: ", PasswordFeedback::Mask('*')) {
        Ok(pass) => println!("You entered: {}", pass),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\n=== Mask('#') mode ===");
    match rpassword::prompt_password_with_feedback("Password: ", PasswordFeedback::Mask('#')) {
        Ok(pass) => println!("You entered: {}", pass),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\n=== PartialMask('*', 3) mode ===");
    match rpassword::prompt_password_with_feedback(
        "Password: ",
        PasswordFeedback::PartialMask('*', 3),
    ) {
        Ok(pass) => println!("You entered: {}", pass),
        Err(e) => eprintln!("Error: {}", e),
    }
}
