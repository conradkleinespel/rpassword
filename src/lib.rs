//! This library makes it easy to read passwords in a console application on all platforms, Unix,
//! Windows, WASM, etc.
//!
//! Here's how you can read a password:
//! ```no_run
//! let password = rpassword::read_password().unwrap();
//! println!("Your password is {}", password);
//! ```
//!
//! You can also prompt for a password:
//! ```no_run
//! let password = rpassword::prompt_password("Your password: ").unwrap();
//! println!("Your password is {}", password);
//! ```
//!
//! Finally, in unit tests, you might want to pass a `Cursor`, which implements `BufRead`. In that
//! case, you can use `read_password_from_bufread` and `prompt_password_from_bufread`:
//! ```
//! use std::io::Cursor;
//!
//! let mut mock_input = Cursor::new("my-password\n".as_bytes().to_owned());
//! let password = rpassword::read_password_from_bufread(&mut mock_input).unwrap();
//! println!("Your password is {}", password);
//!
//! let mut mock_input = Cursor::new("my-password\n".as_bytes().to_owned());
//! let mut mock_output = Cursor::new(Vec::new());
//! let password = rpassword::prompt_password_from_bufread(&mut mock_input, &mut mock_output, "Your password: ").unwrap();
//! println!("Your password is {}", password);
//! ```

use rtoolbox::fix_line_issues::fix_line_issues;
use rtoolbox::print_tty::{print_tty, print_writer};
use rtoolbox::safe_string::SafeString;
use std::io::{BufRead, Write};

/// Controls visual feedback when the user types a password.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PasswordFeedback {
    /// Show nothing while typing (current default behavior).
    Hide,
    /// Show the given mask char for every character typed.
    /// e.g. `Mask('*')` shows stars.
    Mask(char),
    /// Show the actual character for the first N chars, then the given
    /// mask char for the rest.
    /// e.g. `PartialMask('*', 3)` shows first 3 chars in plaintext, then stars.
    PartialMask(char, usize),
}

impl Default for PasswordFeedback {
    fn default() -> Self {
        PasswordFeedback::Hide
    }
}

/// Configuration for prompting and reading a password.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub feedback: PasswordFeedback,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            feedback: PasswordFeedback::Hide,
        }
    }
}

/// A builder for creating a [`Config`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ConfigBuilder {
    feedback: PasswordFeedback,
}

impl ConfigBuilder {
    /// Creates a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the visual feedback for the password.
    pub fn password_feedback(mut self, feedback: PasswordFeedback) -> Self {
        self.feedback = feedback;
        self
    }

    /// Builds the final [`Config`].
    pub fn build(self) -> Config {
        Config {
            feedback: self.feedback,
        }
    }
}

struct FeedbackState {
    password: SafeString,
    displayed_count: usize,
    feedback: PasswordFeedback,
}

impl FeedbackState {
    fn new(feedback: PasswordFeedback) -> Self {
        FeedbackState {
            password: SafeString::new(),
            displayed_count: 0,
            feedback,
        }
    }

    fn push_char(&mut self, c: char) -> Vec<u8> {
        self.password.push(c);
        match self.feedback {
            PasswordFeedback::Hide => Vec::new(),
            PasswordFeedback::Mask(mask) => {
                self.displayed_count += 1;
                char_to_bytes(mask)
            }
            PasswordFeedback::PartialMask(mask, n) => {
                self.displayed_count += 1;
                if self.displayed_count <= n {
                    char_to_bytes(c)
                } else {
                    char_to_bytes(mask)
                }
            }
        }
    }

    fn pop_char(&mut self) -> Vec<u8> {
        let last_char = self.password.chars().last();
        if let Some(c) = last_char {
            let new_len = self.password.len() - c.len_utf8();
            self.password.truncate(new_len);

            if self.displayed_count > 0 {
                self.displayed_count -= 1;
                vec![0x08, b' ', 0x08]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    fn clear(&mut self) -> Vec<u8> {
        let count = self.displayed_count;
        self.password = SafeString::new();
        self.displayed_count = 0;
        [0x08u8, b' ', 0x08].repeat(count)
    }

    fn is_empty(&self) -> bool {
        self.password.is_empty()
    }

    fn into_password(self) -> String {
        self.password.into_inner()
    }
}

fn char_to_bytes(c: char) -> Vec<u8> {
    let mut buf = [0u8; 4];
    c.encode_utf8(&mut buf).as_bytes().to_vec()
}

#[cfg(target_family = "wasm")]
mod wasm {
    use super::{Config, FeedbackState, PasswordFeedback, SafeString};
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
        let mut password = SafeString::new();

        reader.read_line(&mut password)?;
        super::fix_line_issues(password.into_inner())
    }

    /// Reads a password from TTY using the given config
    pub fn read_password_with_config(config: Config) -> std::io::Result<String> {
        match config.feedback {
            PasswordFeedback::Hide => read_password(),
            // WASM lacks termios; char-by-char reading with echo control is unsupported.
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "password feedback is not supported on WASM",
            )),
        }
    }
}

#[cfg(target_family = "unix")]
mod unix {
    use super::{Config, FeedbackState, PasswordFeedback, SafeString};
    use libc::{c_int, tcsetattr, termios, ECHO, ECHONL, ICANON, ISIG, TCSANOW, VMIN, VTIME};
    use std::io::{self, BufRead, Write};
    use std::mem;
    use std::os::unix::io::AsRawFd;

    const BACKSPACE: u8 = 0x08;
    const DEL: u8 = 0x7F;
    const CTRL_C: u8 = 0x03;
    const CTRL_D: u8 = 0x04;
    const CTRL_U: u8 = 0x15;
    const ESC: u8 = 0x1B;

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
            // Set the mode back to normal
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
        let mut password = SafeString::new();

        let hidden_input = HiddenInput::new(fd)?;

        reader.read_line(&mut password)?;

        std::mem::drop(hidden_input);

        super::fix_line_issues(password.into_inner())
    }

    struct RawModeInput {
        fd: i32,
        term_orig: termios,
    }

    impl RawModeInput {
        fn new(fd: i32) -> io::Result<RawModeInput> {
            let mut term = safe_tcgetattr(fd)?;
            let term_orig = safe_tcgetattr(fd)?;

            term.c_lflag &= !(ECHO | ICANON | ECHONL | ISIG);
            term.c_cc[VMIN] = 1;
            term.c_cc[VTIME] = 0;

            io_result(unsafe { tcsetattr(fd, TCSANOW, &term) })?;

            Ok(RawModeInput { fd, term_orig })
        }
    }

    impl Drop for RawModeInput {
        fn drop(&mut self) {
            unsafe {
                tcsetattr(self.fd, TCSANOW, &self.term_orig);
            }
        }
    }

    /// Reads a password from TTY using the given config
    pub fn read_password_with_config(config: Config) -> std::io::Result<String> {
        if config.feedback == PasswordFeedback::Hide {
            return read_password();
        }

        let mut tty = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")?;
        let fd = tty.as_raw_fd();

        let raw = RawModeInput::new(fd)?;
        let mut state = FeedbackState::new(config.feedback);
        let mut byte = [0u8; 1];

        loop {
            let n = unsafe { libc::read(fd, byte.as_mut_ptr() as *mut libc::c_void, 1) };
            if n <= 0 {
                return Err(std::io::Error::last_os_error());
            }

            match byte[0] {
                // LF / CR (Enter)
                b'\n' | b'\r' => {
                    tty.write_all(b"\n")?;
                    tty.flush()?;
                    break;
                }
                // Backspace / DEL
                DEL | BACKSPACE => {
                    let output = state.pop_char();
                    if !output.is_empty() {
                        tty.write_all(&output)?;
                        tty.flush()?;
                    }
                }
                // Ctrl-U: clear line
                CTRL_U => {
                    let output = state.clear();
                    if !output.is_empty() {
                        tty.write_all(&output)?;
                        tty.flush()?;
                    }
                }
                // Ctrl-C: interrupt
                CTRL_C => {
                    tty.write_all(b"\n")?;
                    tty.flush()?;
                    std::mem::drop(raw);
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
                    let n = unsafe { libc::read(fd, byte.as_mut_ptr() as *mut libc::c_void, 1) };
                    if n > 0 && (byte[0] == b'[' || byte[0] == b'O') {
                        // CSI (ESC [) or SS3 (ESC O): read until final byte (0x40-0x7E)
                        loop {
                            let n = unsafe {
                                libc::read(fd, byte.as_mut_ptr() as *mut libc::c_void, 1)
                            };
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
                        tty.write_all(&output)?;
                        tty.flush()?;
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
                        let n =
                            unsafe { libc::read(fd, byte.as_mut_ptr() as *mut libc::c_void, 1) };
                        if n <= 0 {
                            break;
                        }
                        utf8_buf.push(byte[0]);
                    }
                    if let Ok(s) = std::str::from_utf8(&utf8_buf) {
                        if let Some(c) = s.chars().next() {
                            let output = state.push_char(c);
                            if !output.is_empty() {
                                tty.write_all(&output)?;
                                tty.flush()?;
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

#[cfg(target_family = "windows")]
mod windows {
    use super::{Config, FeedbackState, PasswordFeedback, SafeString};
    use std::io::{self, BufRead, BufReader, Write};
    use std::os::windows::io::FromRawHandle;
    use windows_sys::core::PCSTR;
    use windows_sys::Win32::Foundation::{
        GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileA, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, ReadConsoleW, SetConsoleMode, CONSOLE_MODE, ENABLE_LINE_INPUT,
        ENABLE_PROCESSED_INPUT,
    };

    const BACKSPACE: char = '\x08';
    const DEL: char = '\x7F';
    const CTRL_C: char = '\x03';
    const CTRL_D: char = '\x04';
    const CTRL_U: char = '\x15';
    const ESC: char = '\x1B';

    struct HiddenInput {
        mode: u32,
        handle: HANDLE,
    }

    impl HiddenInput {
        fn new(handle: HANDLE) -> io::Result<HiddenInput> {
            let mut mode = 0;

            // Get the old mode so we can reset back to it when we are done
            if unsafe { GetConsoleMode(handle, &mut mode as *mut CONSOLE_MODE) } == 0 {
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
            // Set the mode back to normal
            unsafe {
                SetConsoleMode(self.handle, self.mode);
            }
        }
    }

    /// Reads a password from the TTY
    pub fn read_password() -> std::io::Result<String> {
        let handle = unsafe {
            CreateFileA(
                b"CONIN$\x00".as_ptr() as PCSTR,
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                INVALID_HANDLE_VALUE,
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }

        let mut stream = BufReader::new(unsafe { std::fs::File::from_raw_handle(handle as _) });
        read_password_from_handle_with_hidden_input(&mut stream, handle)
    }

    /// Reads a password from a given file handle
    fn read_password_from_handle_with_hidden_input(
        reader: &mut impl BufRead,
        handle: HANDLE,
    ) -> io::Result<String> {
        let mut password = SafeString::new();

        let hidden_input = HiddenInput::new(handle)?;

        let reader_return = reader.read_line(&mut password);

        // Newline for windows which otherwise prints on the same line.
        println!();

        if reader_return.is_err() {
            return Err(reader_return.unwrap_err());
        }

        std::mem::drop(hidden_input);

        super::fix_line_issues(password.into_inner())
    }

    struct RawModeInput {
        mode: u32,
        handle: HANDLE,
    }

    impl RawModeInput {
        fn new(handle: HANDLE) -> io::Result<RawModeInput> {
            let mut mode = 0;

            if unsafe { GetConsoleMode(handle, &mut mode as *mut CONSOLE_MODE) } == 0 {
                return Err(std::io::Error::last_os_error());
            }

            if unsafe { SetConsoleMode(handle, ENABLE_PROCESSED_INPUT) } == 0 {
                return Err(std::io::Error::last_os_error());
            }

            Ok(RawModeInput { mode, handle })
        }
    }

    impl Drop for RawModeInput {
        fn drop(&mut self) {
            unsafe {
                SetConsoleMode(self.handle, self.mode);
            }
        }
    }

    /// Reads a password from TTY using the given config
    pub fn read_password_with_config(config: Config) -> std::io::Result<String> {
        if config.feedback == PasswordFeedback::Hide {
            return read_password();
        }

        let in_handle = unsafe {
            CreateFileA(
                b"CONIN$\x00".as_ptr() as PCSTR,
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                INVALID_HANDLE_VALUE,
            )
        };
        if in_handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }

        let out_handle = unsafe {
            CreateFileA(
                b"CONOUT$\x00".as_ptr() as PCSTR,
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                INVALID_HANDLE_VALUE,
            )
        };
        if out_handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }

        let mut out_stream = unsafe { std::fs::File::from_raw_handle(out_handle as _) };

        let _raw = RawModeInput::new(in_handle)?;
        let mut state = FeedbackState::new(config.feedback);

        loop {
            let mut buf: [u16; 1] = [0];
            let mut chars_read: u32 = 0;
            if unsafe {
                ReadConsoleW(
                    in_handle,
                    buf.as_mut_ptr() as *mut std::ffi::c_void,
                    1,
                    &mut chars_read,
                    std::ptr::null(),
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            if chars_read == 0 {
                continue;
            }

            let wchar = buf[0];

            // Handle UTF-16 surrogate pairs: characters above U+FFFF (e.g. emoji)
            // are split across two u16 values — a high surrogate (0xD800..0xDBFF)
            // followed by a low surrogate. Read the second half before decoding.
            let c = if (0xD800..=0xDBFF).contains(&wchar) {
                let mut buf2: [u16; 1] = [0];
                let mut chars_read2: u32 = 0;
                if unsafe {
                    ReadConsoleW(
                        in_handle,
                        buf2.as_mut_ptr() as *mut std::ffi::c_void,
                        1,
                        &mut chars_read2,
                        std::ptr::null(),
                    )
                } == 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                match char::decode_utf16([wchar, buf2[0]])
                    .next()
                    .and_then(|r| r.ok())
                {
                    Some(c) => c,
                    // Invalid/mismatched surrogate pair; shouldn't happen with
                    // ReadConsoleW but we skip gracefully rather than panicking.
                    None => continue,
                }
            } else {
                match char::from_u32(wchar as u32) {
                    Some(c) => c,
                    // Orphaned surrogate (0xD800-0xDFFF) as a lone u16; defensive
                    // guard since ReadConsoleW shouldn't produce these.
                    None => continue,
                }
            };

            match c {
                // LF / CR (Enter)
                '\n' | '\r' => {
                    out_stream.write_all(b"\n")?;
                    out_stream.flush()?;
                    break;
                }
                // Backspace / DEL
                DEL | BACKSPACE => {
                    let output = state.pop_char();
                    if !output.is_empty() {
                        out_stream.write_all(&output)?;
                        out_stream.flush()?;
                    }
                }
                // Ctrl-U: clear line
                CTRL_U => {
                    let output = state.clear();
                    if !output.is_empty() {
                        out_stream.write_all(&output)?;
                        out_stream.flush()?;
                    }
                }
                // Ctrl-C: interrupt
                CTRL_C => {
                    out_stream.write_all(b"\n")?;
                    out_stream.flush()?;
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
                    let mut buf3: [u16; 1] = [0];
                    let mut chars_read3: u32 = 0;
                    let ok = unsafe {
                        ReadConsoleW(
                            in_handle,
                            buf3.as_mut_ptr() as *mut std::ffi::c_void,
                            1,
                            &mut chars_read3,
                            std::ptr::null(),
                        )
                    } != 0;
                    if ok && (buf3[0] == b'[' as u16 || buf3[0] == b'O' as u16) {
                        // CSI (ESC [) or SS3 (ESC O): read until final byte (0x40-0x7E)
                        loop {
                            let mut buf4: [u16; 1] = [0];
                            let mut chars_read4: u32 = 0;
                            if unsafe {
                                ReadConsoleW(
                                    in_handle,
                                    buf4.as_mut_ptr() as *mut std::ffi::c_void,
                                    1,
                                    &mut chars_read4,
                                    std::ptr::null(),
                                )
                            } == 0
                            {
                                break;
                            }
                            if (0x40..=0x7E).contains(&buf4[0]) {
                                break;
                            }
                        }
                    }
                    // Otherwise: 2-byte sequence (ESC + char), already consumed
                }
                c if c >= ' ' && !c.is_control() => {
                    let output = state.push_char(c);
                    if !output.is_empty() {
                        out_stream.write_all(&output)?;
                        out_stream.flush()?;
                    }
                }
                // Discard unrecognized control characters and invalid input
                _ => {}
            }
        }

        Ok(state.into_password())
    }
}

#[cfg(target_family = "unix")]
pub use unix::read_password;
#[cfg(target_family = "unix")]
pub use unix::read_password_with_config;
#[cfg(target_family = "wasm")]
pub use wasm::read_password;
#[cfg(target_family = "wasm")]
pub use wasm::read_password_with_config;
#[cfg(target_family = "windows")]
pub use windows::read_password;
#[cfg(target_family = "windows")]
pub use windows::read_password_with_config;

/// Reads a password from `impl BufRead`
pub fn read_password_from_bufread(reader: &mut impl BufRead) -> std::io::Result<String> {
    let mut password = SafeString::new();
    reader.read_line(&mut password)?;

    fix_line_issues(password.into_inner())
}

/// Prompts on `impl Write` and then reads a password from `impl BufRead`
pub fn prompt_password_from_bufread(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    prompt: impl ToString,
) -> std::io::Result<String> {
    print_writer(writer, prompt.to_string().as_str())
        .and_then(|_| read_password_from_bufread(reader))
}

/// Prompts on the TTY and then reads a password from TTY
pub fn prompt_password(prompt: impl ToString) -> std::io::Result<String> {
    print_tty(prompt.to_string().as_str()).and_then(|_| read_password())
}

/// Prompts on the TTY and then reads a password from TTY using the given config
pub fn prompt_password_with_config(
    prompt: impl ToString,
    config: Config,
) -> std::io::Result<String> {
    print_tty(prompt.to_string().as_str()).and_then(|_| read_password_with_config(config))
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let response = read_password_from_bufread(&mut reader_crlf).unwrap();
        assert_eq!(response, "A mocked response.");
        let response = read_password_from_bufread(&mut reader_crlf).unwrap();
        assert_eq!(response, "Another mocked response.");

        let mut reader_lf = mock_input_lf();
        let response = read_password_from_bufread(&mut reader_lf).unwrap();
        assert_eq!(response, "A mocked response.");
        let response = read_password_from_bufread(&mut reader_lf).unwrap();
        assert_eq!(response, "Another mocked response.");
    }

    #[test]
    fn feedback_state_mask_star() {
        let mut state = FeedbackState::new(PasswordFeedback::Mask('*'));
        assert_eq!(state.push_char('a'), b"*");
        assert_eq!(state.push_char('b'), b"*");
        assert_eq!(state.push_char('c'), b"*");
        assert_eq!(state.pop_char(), vec![0x08, b' ', 0x08]);
        assert_eq!(state.into_password(), "ab");
    }

    #[test]
    fn feedback_state_mask_hash() {
        let mut state = FeedbackState::new(PasswordFeedback::Mask('#'));
        assert_eq!(state.push_char('x'), b"#");
        assert_eq!(state.push_char('y'), b"#");
        assert_eq!(state.into_password(), "xy");
    }

    #[test]
    fn feedback_state_hide() {
        let mut state = FeedbackState::new(PasswordFeedback::Hide);
        assert!(state.push_char('a').is_empty());
        assert!(state.push_char('b').is_empty());
        assert!(state.pop_char().is_empty());
        assert_eq!(state.into_password(), "a");
    }

    #[test]
    fn feedback_state_partial_mask() {
        let mut state = FeedbackState::new(PasswordFeedback::PartialMask('*', 3));
        assert_eq!(state.push_char('a'), b"a");
        assert_eq!(state.push_char('b'), b"b");
        assert_eq!(state.push_char('c'), b"c");
        assert_eq!(state.push_char('d'), b"*");
        assert_eq!(state.push_char('e'), b"*");
        assert_eq!(state.into_password(), "abcde");
    }

    #[test]
    fn feedback_state_backspace_empty() {
        let mut state = FeedbackState::new(PasswordFeedback::Mask('*'));
        assert!(state.pop_char().is_empty());
    }

    #[test]
    fn feedback_state_clear() {
        let mut state = FeedbackState::new(PasswordFeedback::Mask('*'));
        state.push_char('a');
        state.push_char('b');
        state.push_char('c');
        let erase = state.clear();
        assert_eq!(erase, [0x08u8, b' ', 0x08].repeat(3));
        assert!(state.is_empty());
    }

    #[test]
    fn feedback_state_partial_mask_zero() {
        let mut state = FeedbackState::new(PasswordFeedback::PartialMask('*', 0));
        assert_eq!(state.push_char('a'), b"*");
        assert_eq!(state.push_char('b'), b"*");
        assert_eq!(state.into_password(), "ab");
    }
}
