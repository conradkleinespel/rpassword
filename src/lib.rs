// Copyright 2014-2017 The Rpassword Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[cfg(unix)]
extern crate libc;
extern crate rprompt;

#[cfg(windows)]
extern crate winapi;

use std::io::Write;
use std::io::{stdin, BufRead};

mod zero_on_drop;
use zero_on_drop::ZeroOnDrop;

/// Removes the \n from the read line
fn fixes_newline(password: &mut ZeroOnDrop) {
    // We may not have a newline, e.g. if user sent CTRL-D or if
    // this is not a TTY.

    if password.ends_with('\n') {
        // Remove the \n from the line if present
        password.pop();

        // Remove the \r from the line if present
        if password.ends_with('\r') {
            password.pop();
        }
    }
}

#[cfg(unix)]
mod unix {
    use libc::{c_int, isatty, tcsetattr, termios, ECHO, ECHONL, STDIN_FILENO, TCSANOW};
    use std::io::{self, BufRead, StdinLock, Write};
    use std::mem;
    use std::os::unix::io::AsRawFd;

    /// Checks if the program is run via a TTY
    #[cfg(unix)]
    pub fn stdin_tty() -> bool {
        unsafe { isatty(STDIN_FILENO) == 1 }
    }

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
    fn io_result(ret: c_int) -> ::std::io::Result<()> {
        match ret {
            0 => Ok(()),
            _ => Err(::std::io::Error::last_os_error()),
        }
    }

    fn safe_tcgetattr(fd: c_int) -> ::std::io::Result<termios> {
        let mut term = mem::MaybeUninit::<::unix::termios>::uninit();
        io_result(unsafe { ::libc::tcgetattr(fd, term.as_mut_ptr()) })?;
        Ok(unsafe { term.assume_init() })
    }

    /// Reads a password from the TTY
    pub fn read_password_from_tty() -> ::std::io::Result<String> {
        let tty = ::std::fs::File::open("/dev/tty")?;
        let fd = tty.as_raw_fd();
        let mut source = io::BufReader::new(tty);

        read_password_from_fd(&mut source, fd)
    }

    /// Reads a password from an existing StdinLock
    pub fn read_password_from_stdin_lock(reader: &mut StdinLock) -> ::std::io::Result<String> {
        if ::stdin_tty() {
            read_password_from_fd(reader, STDIN_FILENO)
        } else {
            ::read_password_from_bufread(reader)
        }
    }

    /// Reads a password from a given file descriptor
    fn read_password_from_fd(reader: &mut impl BufRead, fd: i32) -> ::std::io::Result<String> {
        let mut password = super::ZeroOnDrop::new();

        let mut hidden_input = HiddenInput::new(fd)?;

        reader.read_line(&mut password)?;
        super::fixes_newline(&mut password);

        std::mem::drop(hidden_input);

        Ok(password.into_inner())
    }
}

#[cfg(windows)]
mod windows {
    use std::io::{self, BufReader, Cursor, Write};
    use std::io::{BufRead, Read, StdinLock};
    use std::os::windows::io::{AsRawHandle, FromRawHandle};
    use std::ptr;
    use winapi::ctypes::c_void;
    use winapi::shared::minwindef::{DWORD, LPDWORD, LPVOID};
    use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
    use winapi::um::fileapi::{CreateFileA, GetFileType, ReadFile, OPEN_EXISTING};
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::minwinbase::LPOVERLAPPED;
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::{FILE_TYPE_CHAR, FILE_TYPE_PIPE, STD_INPUT_HANDLE};
    use winapi::um::wincon::{ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT};
    use winapi::um::winnt::{
        FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE, HANDLE, PVOID,
    };

    /// Checks if the program is run via a TTY
    pub fn stdin_tty() -> bool {
        let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        if handle == INVALID_HANDLE_VALUE {
            panic!("Invalid STDIN handle");
        }

        unsafe { GetFileType(handle) == FILE_TYPE_CHAR }
    }

    struct HiddenInput {
        mode: u32,
        handle: HANDLE,
    }

    impl HiddenInput {
        fn new(handle: HANDLE) -> io::Result<HiddenInput> {
            let mut mode = 0;

            // Get the old mode so we can reset back to it when we are done
            if unsafe { GetConsoleMode(handle, &mut mode as LPDWORD) } == 0 {
                return Err(::std::io::Error::last_os_error());
            }

            // We want to be able to read line by line, and we still want backspace to work
            let new_mode_flags = ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT;
            if unsafe { SetConsoleMode(handle, new_mode_flags) } == 0 {
                return Err(::std::io::Error::last_os_error());
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
    pub fn read_password_from_tty() -> ::std::io::Result<String> {
        let handle = unsafe {
            CreateFileA(
                b"CONIN$\x00".as_ptr() as *const i8,
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null_mut(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(::std::io::Error::last_os_error());
        }

        let mut stream = BufReader::new(unsafe { ::std::fs::File::from_raw_handle(handle) });
        read_password_from_handle(&mut stream, handle)
    }

    /// Reads a password from an existing StdinLock
    pub fn read_password_from_stdin_lock(reader: &mut StdinLock) -> ::std::io::Result<String> {
        let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        if handle == INVALID_HANDLE_VALUE {
            return Err(::std::io::Error::last_os_error());
        }

        if unsafe { GetFileType(handle) } == FILE_TYPE_PIPE {
            ::read_password_from_bufread(reader)
        } else {
            read_password_from_handle(reader, handle)
        }
    }

    /// Reads a password from a given file handle
    fn read_password_from_handle(reader: &mut impl BufRead, handle: HANDLE) -> io::Result<String> {
        let mut password = super::ZeroOnDrop::new();

        let hidden_input = HiddenInput::new(handle)?;

        reader.read_line(&mut password)?;
        super::fixes_newline(&mut password);

        // Newline for windows which otherwise prints on the same line.
        println!();

        std::mem::drop(hidden_input);

        Ok(password.into_inner())
    }
}

#[cfg(unix)]
pub use unix::{read_password_from_stdin_lock, read_password_from_tty, stdin_tty};
#[cfg(windows)]
pub use windows::{read_password_from_stdin_lock, read_password_from_tty, stdin_tty};

/// Reads a password from stdin
pub fn read_password() -> ::std::io::Result<String> {
    read_password_from_stdin_lock(&mut stdin().lock())
}

/// Reads a password from anything that implements BufRead
pub fn read_password_from_bufread(source: &mut impl BufRead) -> ::std::io::Result<String> {
    let mut password = ZeroOnDrop::new();
    source.read_line(&mut password)?;

    fixes_newline(&mut password);
    Ok(password.into_inner())
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

        let response = ::read_password_from_bufread(&mut reader_crlf).unwrap();
        assert_eq!(response, "A mocked response.");
        let response = ::read_password_from_bufread(&mut reader_crlf).unwrap();
        assert_eq!(response, "Another mocked response.");

        let mut reader_lf = mock_input_lf();
        let response = ::read_password_from_bufread(&mut reader_lf).unwrap();
        assert_eq!(response, "A mocked response.");
        let response = ::read_password_from_bufread(&mut reader_lf).unwrap();
        assert_eq!(response, "Another mocked response.");
    }
}
