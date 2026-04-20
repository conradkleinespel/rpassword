use rpassword::{ConfigBuilder, InputOutput, InputOutputTarget, PasswordFeedback};

fn main() {
    println!("=== prompt_password_with_config(...) with no input ===");
    let config = ConfigBuilder::new()
        .password_feedback(PasswordFeedback::Hide)
        .input_output(InputOutput::new().input(InputOutputTarget::Void))
        .build();
    // Adding an extra \n so the output looks somewhat understandable with `input(Void)`
    match rpassword::prompt_password_with_config("Password: \n", config) {
        Ok(pass) => println!("You entered: '{}' (empty because Void input target)", pass),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("=== prompt_password_with_config(...) with no output ===");
    println!("Note: No prompt will be shown, since the output is discarded.");
    println!("Just type your password and press Enter.");
    let config = ConfigBuilder::new()
        .password_feedback(PasswordFeedback::Hide)
        .input_output(InputOutput::new().output(InputOutputTarget::Void))
        .build();
    match rpassword::prompt_password_with_config("Password: ", config) {
        Ok(pass) => println!("You entered: '{}'", pass),
        Err(e) => eprintln!("Error: {}", e),
    }
}
