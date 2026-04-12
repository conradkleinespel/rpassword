use rpassword::{ConfigBuilder, PasswordFeedback};

fn main() {
    println!("=== Hide mode (default behavior) ===");
    let config = ConfigBuilder::new()
        .password_feedback(PasswordFeedback::Hide)
        .build();
    match rpassword::prompt_password_with_config("Password: ", config) {
        Ok(pass) => println!("You entered: {}", pass),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\n=== Mask('*') mode ===");
    let config = ConfigBuilder::new()
        .password_feedback(PasswordFeedback::Mask('*'))
        .build();
    match rpassword::prompt_password_with_config("Password: ", config) {
        Ok(pass) => println!("You entered: {}", pass),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\n=== Mask('#') mode ===");
    let config = ConfigBuilder::new()
        .password_feedback(PasswordFeedback::Mask('#'))
        .build();
    match rpassword::prompt_password_with_config("Password: ", config) {
        Ok(pass) => println!("You entered: {}", pass),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\n=== PartialMask('*', 3) mode ===");
    let config = ConfigBuilder::new()
        .password_feedback(PasswordFeedback::PartialMask('*', 3))
        .build();
    match rpassword::prompt_password_with_config("Password: ", config) {
        Ok(pass) => println!("You entered: {}", pass),
        Err(e) => eprintln!("Error: {}", e),
    }
}
