use libc::{c_int, isatty, tcsetattr, termios, ECHO, ECHONL, ICANON, ISIG, TCSANOW, VMIN, VTIME};
use std::fs::File;
use std::io::{self, Read, Write};
use std::mem;
use std::os::unix::io::AsRawFd;
use crate::config::{Config, PasswordFeedback};
use crate::feedback::FeedbackState;

pub const DEFAULT_INPUT_PATH: &str = "/dev/tty";
pub const DEFAULT_OUTPUT_PATH: &str = "/dev/tty";

const BACKSPACE: u8 = 0x08;
const DEL: u8 = 0x7F;
const CTRL_C: u8 = 0x03;
const CTRL_D: u8 = 0x04;
const CTRL_U: u8 = 0x15;
const ESC: u8 = 0x1B;

/// Turns a C function return into an IO Result
fn io_result(ret: c_int) -> std::io::Result<()> {
    match ret {
        0 => Ok(()),
        _ => Err(std::io::Error::last_os_error()),
    }
}

fn is_interactive_terminal(fd: c_int) -> bool {
    let result = unsafe {
        isatty(fd) != 0
    };
    // For any non terminal, `isatty` produces ENOTTY, we clean it up
    unsafe {
        *libc::__errno_location() = 0;
    };
    result
}

fn safe_tcgetattr(fd: c_int) -> std::io::Result<termios> {
    let mut term = mem::MaybeUninit::<termios>::uninit();
    io_result(unsafe { ::libc::tcgetattr(fd, term.as_mut_ptr()) })?;
    Ok(unsafe { term.assume_init() })
}

fn safe_tcsetattr(fd: c_int, term: &mut termios) -> std::io::Result<()> {
    io_result(unsafe {tcsetattr(fd, TCSANOW, term)})
}

#[derive(Debug)]
struct RawModeInput {
    input_file: File,
    output_file: File,
    term_orig: Option<termios>,
    needs_terminal_configuration: bool,
    password_feedback: PasswordFeedback,
}

impl Drop for RawModeInput {
    fn drop(&mut self) {
        if let Some(ref mut term_orig) = self.term_orig {
            unsafe {
                tcsetattr(self.input_file.as_raw_fd(), TCSANOW, term_orig);
            }
        }
    }
}

impl RawModeInput {
    fn new(config: Config) -> io::Result<RawModeInput> {
        let input_file = std::fs::OpenOptions::new()
            .read(true)
            .open(config.input_path.as_str())?;
        let input_fd = input_file.as_raw_fd();
        let is_a_tty = is_interactive_terminal(input_fd);

        let output_file = std::fs::OpenOptions::new()
            .write(true)
            .open(config.output_path.as_str())?;

        Ok(RawModeInput {
            input_file,
            output_file,
            term_orig: if is_a_tty { Some(safe_tcgetattr(input_fd)?) } else { None },
            needs_terminal_configuration: is_a_tty,
            password_feedback: config.feedback,
        })
    }

    fn apply_terminal_configuration(&mut self) -> io::Result<()> {
        if !self.needs_terminal_configuration {
            panic!("apply_terminal_configuration called on non-TTY");
        }

        let mut term = safe_tcgetattr(self.input_file.as_raw_fd())?;
        term.c_lflag &= !(ECHO | ICANON | ECHONL | ISIG);
        term.c_cc[VMIN] = 1;
        term.c_cc[VTIME] = 0;
        safe_tcsetattr(self.input_file.as_raw_fd(), &mut term)
    }

    fn read_password(&mut self) -> std::io::Result<String> {
        if self.needs_terminal_configuration {
            self.apply_terminal_configuration()?;
        }

        let mut state = FeedbackState::new(self.password_feedback, self.needs_terminal_configuration);
        let mut byte = [0u8; 1];

        loop {
            let n = self.input_file.read(&mut byte)?;
            if n <= 0 {
                // EOF
                break;
            }

            match byte[0] {
                // LF / CR (Enter)
                b'\n' | b'\r' => {
                    let output = state.finish();
                    if !output.is_empty() {
                        self.output_file.write_all(&output)?;
                        self.output_file.flush()?;
                    }
                    break;
                }
                // Backspace / DEL
                DEL | BACKSPACE => {
                    let output = state.pop_char();
                    if !output.is_empty() {
                        self.output_file.write_all(&output)?;
                        self.output_file.flush()?;
                    }
                }
                // Ctrl-U: clear line
                CTRL_U => {
                    let output = state.clear();
                    if !output.is_empty() {
                        self.output_file.write_all(&output)?;
                        self.output_file.flush()?;
                    }
                }
                // Ctrl-C: interrupt
                CTRL_C => {
                    let output = state.abort();
                    if !output.is_empty() {
                        self.output_file.write_all(&output)?;
                        self.output_file.flush()?;
                    }
                    unsafe {
                        libc::raise(libc::SIGINT);
                    }
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "interrupted",
                    ));
                }
                // Ctrl-D: EOF when empty
                CTRL_D => {
                    if state.is_empty() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "unexpected end of file",
                        ));
                    }
                }
                // ESC: consume and discard escape sequence
                ESC => {
                    let n = self.input_file.read(&mut byte)?;
                    if n <= 0 {
                        // EOF
                        break;
                    }
                    if n > 0 && (byte[0] == b'[' || byte[0] == b'O') {
                        // CSI (ESC [) or SS3 (ESC O): read until final byte (0x40-0x7E)
                        loop {
                            let n = self.input_file.read(&mut byte)?;
                            if n <= 0 {
                                break;
                            }
                            if (0x40..=0x7E).contains(&byte[0]) {
                                break;
                            }
                        }
                    }
                    // Otherwise: 2-byte sequence (ESC + char), already consumed
                }
                // Printable ASCII
                0x20..=0x7E => {
                    let output = state.push_char(byte[0] as char);
                    if !output.is_empty() {
                        self.output_file.write_all(&output)?;
                        self.output_file.flush()?;
                    }
                }
                // UTF-8 multi-byte lead byte
                0xC0..=0xF7 => {
                    let width = match byte[0] {
                        0xC0..=0xDF => 2,
                        0xE0..=0xEF => 3,
                        0xF0..=0xF7 => 4,
                        _ => unreachable!(),
                    };
                    let mut utf8_buf = vec![byte[0]];
                    for _ in 1..width {
                        let n = self.input_file.read(&mut byte)?;
                        if n <= 0 {
                            break;
                        }
                        utf8_buf.push(byte[0]);
                    }
                    if let Ok(s) = std::str::from_utf8(&utf8_buf) {
                        if let Some(c) = s.chars().next() {
                            let output = state.push_char(c);
                            if !output.is_empty() {
                                self.output_file.write_all(&output)?;
                                self.output_file.flush()?;
                            }
                        }
                    }
                }
                // Discard unrecognized control characters (Ctrl-A, Ctrl-B, etc.)
                // and invalid bytes (bad UTF-8 continuations, 0xF8-0xFF)
                _ => {}
            }
        }

        Ok(state.into_password())
    }
}

/// Reads a password from TTY using the given config
pub fn read_password_with_config(config: Config) -> std::io::Result<String> {
    let mut raw_mode_input = RawModeInput::new(config)?;
    raw_mode_input.read_password()
}

/// Reads a password from the TTY
pub fn read_password() -> std::io::Result<String> {
    read_password_with_config(Config::default())
}


#[cfg(test)]
mod tests {
    use std::io::Write;
    use crate::config::{ConfigBuilder, InputOutput};
    use crate::read_password_with_config;

    #[test]
    fn test_read_password_with_config() {
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(b"password\n").unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();

        let config = ConfigBuilder::new()
            .input_output(InputOutput::InputOutputCombined(path.clone()))
            .build();

        // This should fail because it's not a TTY (tcgetattr fails on regular files)
        // But it proves that read_password_with_config is using our input path.
        let result = read_password_with_config(config);
        assert_eq!("password", result.unwrap());

        // Check that no content was written to the file because it is not a TTY
        let file_content = std::fs::read_to_string(path).unwrap();
        assert_eq!("password\n", file_content);
    }

    #[test]
    fn test_read_password_with_config_errors_with_file_not_found() {
        let config = ConfigBuilder::new()
            .input_output(InputOutput::InputOutputCombined("/does/not/exist".to_string()))
            .build();

        // This should fail because it's not a TTY (tcgetattr fails on regular files)
        // But it proves that read_password_with_config is using our input path.
        let result = read_password_with_config(config);
        assert!(result.is_err());

        // On Linux, tcgetattr on a regular file returns ENOTTY (Inappropriate ioctl for device)
        let err = result.unwrap_err();
        assert_eq!(err.raw_os_error(), Some(libc::ENOENT));
    }
}