use std::io::{self, BufRead};
use rtoolbox::safe_string::SafeString;
use crate::config::{Config, ConfigBuilder, PasswordFeedback};

pub const DEFAULT_INPUT_PATH: &str = "/dev/tty";
pub const DEFAULT_OUTPUT_PATH: &str = "/dev/tty";

/// Reads a password from the TTY
pub fn read_password() -> std::io::Result<String> {
    read_password_with_config(ConfigBuilder::new().build())
}

/// Reads a password from TTY using the given config
pub fn read_password_with_config(config: Config) -> std::io::Result<String> {
    let tty = std::fs::File::open(config.input_path.as_str())?;
    let mut reader = io::BufReader::new(tty);

    match config.feedback {
        PasswordFeedback::Hide => {
            let mut password = SafeString::new();

            reader.read_line(&mut password)?;
            super::fix_line_issues(password.into_inner())
        },
        // WASM lacks termios; char-by-char reading with echo control is unsupported.
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "password feedback is not supported on WASM",
        )),
    }
}