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

extern crate rprompt;
#[cfg(unix)]
extern crate libc;

use std::io::Write;
use std::io::stdin;
use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;

/// Sets all bytes of a String to 0
fn zero_memory(s: &mut String) {
    let mut vec = unsafe { s.as_mut_vec() };
    for el in vec.iter_mut() {
        *el = 0u8;
    }
}

/// Removes the \n from the read line
fn fixes_newline(mut password: String) -> std::io::Result<String> {
    // We should have a newline at the end. This helps prevent things such as:
    // > printf "no-newline" | rpassword
    // If we didn't have the \n check, we'd be removing the last "e" by mistake.
    if password.chars().last() != Some('\n') {
        return Err(IoError::new(
            IoErrorKind::UnexpectedEof,
            "unexpected end of file",
        ));
    }

    // Remove the \n from the line.
    password.pop();

    // Remove the \r from the line if present
    if password.chars().last() == Some('\r') {
        password.pop();
    }

    Ok(password)
}

#[cfg(unix)]
mod unix {
    use libc::{c_int, isatty, tcgetattr, tcsetattr, TCSANOW, ECHO, ECHONL, STDIN_FILENO};

    /// Turns a C function return into an IO Result
    fn io_result(ret: c_int) -> ::std::io::Result<()> {
        match ret {
            0 => Ok(()),
            _ => Err(::std::io::Error::last_os_error()),
        }
    }

    /// Reads a password from STDIN
    pub fn read_password() -> ::std::io::Result<String> {
        let mut password = String::new();

        let input_is_tty = unsafe { isatty(0) } == 1;

        // When we ask for a password in a terminal, we'll want to hide the password as it is
        // typed by the user
        if input_is_tty {
            // Make two copies of the terminal settings. The first one will be modified
            // and the second one will act as a backup for when we want to set the
            // terminal back to its original state.
            let mut term = unsafe { ::std::mem::uninitialized() };
            let mut term_orig = unsafe { ::std::mem::uninitialized() };
            io_result(unsafe { tcgetattr(STDIN_FILENO, &mut term) })?;
            io_result(unsafe { tcgetattr(STDIN_FILENO, &mut term_orig) })?;

            // Hide the password. This is what makes this function useful.
            term.c_lflag &= !ECHO;

            // But don't hide the NL character when the user hits ENTER.
            term.c_lflag |= ECHONL;

            // Save the settings for now.
            io_result(unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &term) })?;

            // Read the password.
            match super::stdin().read_line(&mut password) {
                Ok(_) => {}
                Err(err) => {
                    // Reset the terminal and quit.
                    io_result(unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &term_orig) })?;

                    super::zero_memory(&mut password);
                    return Err(err);
                }
            };

            // Reset the terminal.
            match io_result(unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &term_orig) }) {
                Ok(_) => {}
                Err(err) => {
                    super::zero_memory(&mut password);
                    return Err(err);
                }
            }
        } else {
            // If we don't have a TTY, the input was piped so we bypass
            // terminal hiding code
            match super::stdin().read_line(&mut password) {
                Ok(_) => {}
                Err(err) => {
                    super::zero_memory(&mut password);
                    return Err(err);
                }
            }
        }

        super::fixes_newline(password)
    }
}

#[cfg(windows)]
mod windows {
    extern crate winapi;
    extern crate kernel32;

    /// Reads a password from STDIN
    pub fn read_password() -> ::std::io::Result<String> {
        let mut password = String::new();

        // Get the stdin handle
        let handle = unsafe { kernel32::GetStdHandle(winapi::STD_INPUT_HANDLE) };
        if handle == winapi::INVALID_HANDLE_VALUE {
            return Err(::std::io::Error::last_os_error());
        }

        // Get the old mode so we can reset back to it when we are done
        let mut mode = 0;
        if unsafe { kernel32::GetConsoleMode(handle, &mut mode as winapi::LPDWORD) } == 0 {
            return Err(::std::io::Error::last_os_error());
        }

        // We want to be able to read line by line, and we still want backspace to work
        let new_mode_flags = winapi::ENABLE_LINE_INPUT | winapi::ENABLE_PROCESSED_INPUT;
        if unsafe { kernel32::SetConsoleMode(handle, new_mode_flags) } == 0 {
            return Err(::std::io::Error::last_os_error());
        }

        // Read the password.
        match super::stdin().read_line(&mut password) {
            Ok(_) => {}
            Err(err) => {
                super::zero_memory(&mut password);
                return Err(err);
            }
        };

        // Set the the mode back to normal
        if unsafe { kernel32::SetConsoleMode(handle, mode) } == 0 {
            return Err(::std::io::Error::last_os_error());
        }

        // Since the newline isn't echo'd we need to do it ourselves
        println!("");

        super::fixes_newline(password)
    }
}

#[cfg(unix)]
pub use unix::read_password;
#[cfg(windows)]
pub use windows::read_password;

#[deprecated(since = "1.0.0", note = "use `rprompt` crate and `rprompt::read_reply` instead")]
pub fn read_response() -> std::io::Result<String> {
    rprompt::read_reply()
}

#[deprecated(since = "1.0.0",
             note = "use `rprompt` crate and `rprompt::prompt_reply_stdout` instead")]
pub fn prompt_response_stdout(prompt: &str) -> std::io::Result<String> {
    rprompt::prompt_reply_stdout(prompt)
}

#[deprecated(since = "1.0.0",
             note = "use `rprompt` crate and `rprompt::prompt_reply_stderr` instead")]
pub fn prompt_response_stderr(prompt: &str) -> std::io::Result<String> {
    rprompt::prompt_reply_stderr(prompt)
}

/// Prompts for a password on STDOUT and reads it from STDIN
pub fn prompt_password_stdout(prompt: &str) -> std::io::Result<String> {
    let mut stdout = std::io::stdout();

    write!(stdout, "{}", prompt)?;
    stdout.flush()?;
    read_password()
}

/// Prompts for a password on STDERR and reads it from STDIN
pub fn prompt_password_stderr(prompt: &str) -> std::io::Result<String> {
    let mut stderr = std::io::stderr();

    write!(stderr, "{}", prompt)?;
    stderr.flush()?;
    read_password()
}
