use std::fs::{OpenOptions};
use std::io::{self, BufReader, BufRead};
use rtoolbox::fix_line_issues::fix_line_issues;
use crate::config::{Config};
use crate::RawPasswordInput;

pub const DEFAULT_INPUT_PATH: &str = "/dev/stdin";
pub const DEFAULT_OUTPUT_PATH: &str = "/dev/stdout";

pub struct RawModeInput {
    config: Config,
}

impl RawPasswordInput for RawModeInput {
    fn new(config: Config) -> io::Result<impl RawPasswordInput> {
        Ok(RawModeInput {
            config
        })
    }

    fn needs_terminal_configuration(&self) -> bool {
        false
    }

    fn apply_terminal_configuration(&mut self) -> io::Result<()> {
        Ok(())
    }

    #[allow(unused)]
    fn read_char(&mut self) -> std::io::Result<char> {
        unimplemented!()
    }

    fn read_password(&mut self, _password_feedback: crate::PasswordFeedback) -> std::io::Result<String> {
        let input_file = OpenOptions::new().read(true).open(self.config.input_path.as_str())?;
        let mut reader = BufReader::new(input_file);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        fix_line_issues(line)
    }

    fn write_output(&mut self, _output: &str) -> std::io::Result<()> {
        Ok(())
    }

    fn send_signal_sigint(&mut self) -> io::Result<()> {
        // Not sure what to do with signals on WASM, so just ignore it for now
        Ok(())
    }
}