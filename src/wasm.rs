use std::io::{self, BufRead};
use rtoolbox::safe_string::SafeString;
use crate::config::{Config, PasswordFeedback};

pub const DEFAULT_INPUT_PATH: &str = "/dev/tty";
pub const DEFAULT_OUTPUT_PATH: &str = "/dev/tty";

pub struct RawModeInput {
    config: Config,
}

impl RawModeInput {
    pub fn new(config: Config) -> io::Result<RawModeInput> {
        Ok(RawModeInput {
            config
        })
    }

    pub fn read_password(&mut self) -> std::io::Result<String> {
        let tty = std::fs::File::open(self.config.input_path.as_str())?;
        let mut reader = io::BufReader::new(tty);

        match self.config.feedback {
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
}