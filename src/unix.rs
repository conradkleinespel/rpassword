use crate::RawPasswordInput;
use crate::config::Config;
use libc::{ECHO, ECHONL, ICANON, ISIG, TCSANOW, VMIN, VTIME, c_int, isatty, tcsetattr, termios};
use std::fs::File;
use std::io::{self, Read, Write};
use std::mem;
use std::os::unix::io::AsRawFd;

pub const DEFAULT_INPUT_PATH: &str = "/dev/tty";
pub const DEFAULT_OUTPUT_PATH: &str = "/dev/tty";

/// Turns a C function return into an IO Result
fn io_result(ret: c_int) -> std::io::Result<()> {
    match ret {
        0 => Ok(()),
        _ => Err(std::io::Error::last_os_error()),
    }
}

fn is_interactive_terminal(fd: c_int) -> bool {
    let result = unsafe { isatty(fd) != 0 };
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
    io_result(unsafe { tcsetattr(fd, TCSANOW, term) })
}

fn read_char(reader: &mut impl Read) -> std::io::Result<char> {
    let mut byte = [0u8; 1];
    let n = reader.read(&mut byte)?;
    if n == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "unexpected end of file",
        ));
    }

    match byte[0] {
        // ASCII
        0x00..=0x7F => Ok(byte[0] as char),
        // UTF-8 lead byte
        0xC0..=0xF7 => {
            let width = match byte[0] {
                0xC0..=0xDF => 2,
                0xE0..=0xEF => 3,
                0xF0..=0xF7 => 4,
                _ => unreachable!(),
            };
            let mut utf8_buf = vec![byte[0]];
            for _ in 1..width {
                let n = reader.read(&mut byte)?;
                if n == 0 {
                    return Ok('\u{FFFD}');
                }
                utf8_buf.push(byte[0]);
            }
            if let Ok(s) = std::str::from_utf8(&utf8_buf) {
                if let Some(c) = s.chars().next() {
                    Ok(c)
                } else {
                    Ok('\u{FFFD}')
                }
            } else {
                Ok('\u{FFFD}')
            }
        }
        // Invalid byte
        _ => Ok('\u{FFFD}'),
    }
}

#[derive(Debug)]
pub struct RawModeInput {
    input_file: File,
    output_file: File,
    term_orig: Option<termios>,
    needs_terminal_configuration: bool,
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

impl RawPasswordInput for RawModeInput {
    fn new(config: Config) -> io::Result<impl RawPasswordInput> {
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
            term_orig: if is_a_tty {
                Some(safe_tcgetattr(input_fd)?)
            } else {
                None
            },
            needs_terminal_configuration: is_a_tty,
        })
    }

    fn needs_terminal_configuration(&self) -> bool {
        self.needs_terminal_configuration
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

    fn read_char(&mut self) -> std::io::Result<char> {
        read_char(&mut self.input_file)
    }

    fn write_output(&mut self, output: &str) -> std::io::Result<()> {
        self.output_file.write_all(output.as_bytes())?;
        self.output_file.flush()
    }

    fn send_signal_sigint(&mut self) -> io::Result<()> {
        if unsafe { libc::raise(libc::SIGINT) != 0 } {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{ConfigBuilder, InputOutput};
    use crate::read_password_with_config;
    use std::io::Write;

    #[test]
    fn test_read_password_with_config() {
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(b"password\n").unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();

        let config = ConfigBuilder::new()
            .input_output(InputOutput::InputOutputCombined(path.clone()))
            .build();

        let result = read_password_with_config(config);
        assert_eq!("password", result.unwrap());

        let file_content = std::fs::read_to_string(path).unwrap();
        assert_eq!("password\n", file_content);
    }

    #[test]
    fn test_read_password_with_config_errors_with_file_not_found() {
        let config = ConfigBuilder::new()
            .input_output(InputOutput::InputOutputCombined(
                "/does/not/exist".to_string(),
            ))
            .build();

        // This should fail because the file does not exist
        let result = read_password_with_config(config);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.raw_os_error(), Some(libc::ENOENT));
    }
}
