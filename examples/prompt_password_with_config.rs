use rpassword::{ConfigBuilder, PasswordFeedback};

fn main() {
    println!("=== prompt_password_with_config(...) with Hide mode (default behavior) ===");
    let config = ConfigBuilder::new()
        .password_feedback(PasswordFeedback::Hide)
        .build();
    match rpassword::prompt_password_with_config("Password: ", config) {
        Ok(pass) => println!("You entered: {}", pass),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\n=== prompt_password_with_config(...) with Mask('*') mode ===");
    let config = ConfigBuilder::new()
        .password_feedback(PasswordFeedback::Mask('*'))
        .build();
    match rpassword::prompt_password_with_config("Password: ", config) {
        Ok(pass) => println!("You entered: {}", pass),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\n=== prompt_password_with_config(...) with Mask('#') mode ===");
    let config = ConfigBuilder::new()
        .password_feedback(PasswordFeedback::Mask('#'))
        .build();
    match rpassword::prompt_password_with_config("Password: ", config) {
        Ok(pass) => println!("You entered: {}", pass),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\n=== prompt_password_with_config(...) with PartialMask('*', 3) mode ===");
    let config = ConfigBuilder::new()
        .password_feedback(PasswordFeedback::PartialMask('*', 3))
        .build();
    match rpassword::prompt_password_with_config("Password: ", config) {
        Ok(pass) => println!("You entered: {}", pass),
        Err(e) => eprintln!("Error: {}", e),
    }
}
