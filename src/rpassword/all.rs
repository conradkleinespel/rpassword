use crate::rutil::fix_new_line::fix_new_line;
use crate::rutil::print_tty::{print_tty, print_writer};
use crate::rutil::safe_string::SafeString;
use std::io::{BufRead, Write};

#[cfg(target_family = "wasm")]
mod wasm {
    use std::io::{self, BufRead};

    /// Reads a password from the TTY
    pub fn read_password() -> std::io::Result<String> {
        let tty = std::fs::File::open("/dev/tty")?;
        let mut reader = io::BufReader::new(tty);

        read_password_from_fd_with_hidden_input(&mut reader)
    }

    /// Reads a password from a given file descriptor
    fn read_password_from_fd_with_hidden_input(
        reader: &mut impl BufRead,
    ) -> std::io::Result<String> {
        let mut password = super::SafeString::new();

        reader.read_line(&mut password)?;
        super::fix_new_line(password.into_inner())
    }
}

#[cfg(target_family = "unix")]
mod unix {
    use libc::{c_int, tcsetattr, termios, ECHO, ECHONL, TCSANOW};
    use std::io::{self, BufRead};
    use std::mem;
    use std::os::unix::io::AsRawFd;

    struct HiddenInput {
        fd: i32,
        term_orig: termios,
    }

    impl HiddenInput {
        fn new(fd: i32) -> io::Result<HiddenInput> {
            // Make two copies of the terminal settings. The first one will be modified
            // and the second one will act as a backup for when we want to set the
            // terminal back to its original state.
            let mut term = safe_tcgetattr(fd)?;
            let term_orig = safe_tcgetattr(fd)?;

            // Hide the password. This is what makes this function useful.
            term.c_lflag &= !ECHO;

            // But don't hide the NL character when the user hits ENTER.
            term.c_lflag |= ECHONL;

            // Save the settings for now.
            io_result(unsafe { tcsetattr(fd, TCSANOW, &term) })?;

            Ok(HiddenInput { fd, term_orig })
        }
    }

    impl Drop for HiddenInput {
        fn drop(&mut self) {
            // Set the the mode back to normal
            unsafe {
                tcsetattr(self.fd, TCSANOW, &self.term_orig);
            }
        }
    }

    /// Turns a C function return into an IO Result
    fn io_result(ret: c_int) -> std::io::Result<()> {
        match ret {
            0 => Ok(()),
            _ => Err(std::io::Error::last_os_error()),
        }
    }

    fn safe_tcgetattr(fd: c_int) -> std::io::Result<termios> {
        let mut term = mem::MaybeUninit::<termios>::uninit();
        io_result(unsafe { ::libc::tcgetattr(fd, term.as_mut_ptr()) })?;
        Ok(unsafe { term.assume_init() })
    }

    /// Reads a password from the TTY
    pub fn read_password() -> std::io::Result<String> {
        let tty = std::fs::File::open("/dev/tty")?;
        let fd = tty.as_raw_fd();
        let mut reader = io::BufReader::new(tty);

        read_password_from_fd_with_hidden_input(&mut reader, fd)
    }

    /// Reads a password from a given file descriptor
    fn read_password_from_fd_with_hidden_input(
        reader: &mut impl BufRead,
        fd: i32,
    ) -> std::io::Result<String> {
        let mut password = super::SafeString::new();

        let hidden_input = HiddenInput::new(fd)?;

        reader.read_line(&mut password)?;

        std::mem::drop(hidden_input);

        super::fix_new_line(password.into_inner())
    }
}

#[cfg(target_family = "windows")]
mod windows {
    use std::io::{self, BufReader};
    use std::io::{BufRead, StdinLock};
    use std::os::windows::io::FromRawHandle;
    use winapi::shared::minwindef::LPDWORD;
    use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
    use winapi::um::fileapi::{CreateFileA, GetFileType, OPEN_EXISTING};
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::{FILE_TYPE_PIPE, STD_INPUT_HANDLE};
    use winapi::um::wincon::{ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT};
    use winapi::um::winnt::{
        FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE, HANDLE,
    };

    struct HiddenInput {
        mode: u32,
        handle: HANDLE,
    }

    impl HiddenInput {
        fn new(handle: HANDLE) -> io::Result<HiddenInput> {
            let mut mode = 0;

            // Get the old mode so we can reset back to it when we are done
            if unsafe { GetConsoleMode(handle, &mut mode as LPDWORD) } == 0 {
                return Err(std::io::Error::last_os_error());
            }

            // We want to be able to read line by line, and we still want backspace to work
            let new_mode_flags = ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT;
            if unsafe { SetConsoleMode(handle, new_mode_flags) } == 0 {
                return Err(std::io::Error::last_os_error());
            }

            Ok(HiddenInput { mode, handle })
        }
    }

    impl Drop for HiddenInput {
        fn drop(&mut self) {
            // Set the the mode back to normal
            unsafe {
                SetConsoleMode(self.handle, self.mode);
            }
        }
    }

    /// Reads a password from the TTY
    pub fn read_password() -> std::io::Result<String> {
        let handle = unsafe {
            CreateFileA(
                b"CONIN$\x00".as_ptr() as *const i8,
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }

        let mut stream = BufReader::new(unsafe { std::fs::File::from_raw_handle(handle) });
        read_password_from_handle_with_hidden_input(&mut stream, handle)
    }

    /// Reads a password from a given file handle
    fn read_password_from_handle_with_hidden_input(
        reader: &mut impl BufRead,
        handle: HANDLE,
    ) -> io::Result<String> {
        let mut password = super::SafeString::new();

        let hidden_input = HiddenInput::new(handle)?;

        reader.read_line(&mut password)?;

        // Newline for windows which otherwise prints on the same line.
        println!();

        std::mem::drop(hidden_input);

        super::fix_new_line(password.into_inner())
    }
}

#[cfg(target_family = "unix")]
pub use unix::read_password;
#[cfg(target_family = "wasm")]
pub use wasm::read_password;
#[cfg(target_family = "windows")]
pub use windows::read_password;

/// Reads a password from anything that implements BufRead
pub fn read_password_from_bufread(reader: &mut impl BufRead) -> std::io::Result<String> {
    let mut password = SafeString::new();
    reader.read_line(&mut password)?;

    fix_new_line(password.into_inner())
}

/// Prompts on the TTY and then reads a password from anything that implements BufRead
pub fn prompt_password_from_bufread(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    prompt: impl ToString,
) -> std::io::Result<String> {
    print_writer(writer, prompt.to_string().as_str())
        .and_then(|_| read_password_from_bufread(reader))
}

/// Prompts on the TTY and then reads a password from stdin
pub fn prompt_password(prompt: impl ToString) -> std::io::Result<String> {
    print_tty(prompt.to_string().as_str()).and_then(|_| read_password())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    fn mock_input_crlf() -> Cursor<&'static [u8]> {
        Cursor::new(&b"A mocked response.\r\nAnother mocked response.\r\n"[..])
    }

    fn mock_input_lf() -> Cursor<&'static [u8]> {
        Cursor::new(&b"A mocked response.\nAnother mocked response.\n"[..])
    }

    #[test]
    fn can_read_from_redirected_input_many_times() {
        let mut reader_crlf = mock_input_crlf();

        let response = crate::rpassword::read_password_from_bufread(&mut reader_crlf).unwrap();
        assert_eq!(response, "A mocked response.");
        let response = crate::rpassword::read_password_from_bufread(&mut reader_crlf).unwrap();
        assert_eq!(response, "Another mocked response.");

        let mut reader_lf = mock_input_lf();
        let response = crate::rpassword::read_password_from_bufread(&mut reader_lf).unwrap();
        assert_eq!(response, "A mocked response.");
        let response = crate::rpassword::read_password_from_bufread(&mut reader_lf).unwrap();
        assert_eq!(response, "Another mocked response.");
    }
}
