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
use std::fmt::Debug;
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    pub feedback: PasswordFeedback,
    pub(crate) input_path: Option<String>,
}

/// A builder for creating a [`Config`].
#[derive(Debug, Clone, Default)]
pub struct ConfigBuilder {
    feedback: PasswordFeedback,
    input_path: Option<String>,
}

impl ConfigBuilder {
    pub fn new() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Sets the visual feedback for the password.
    pub fn password_feedback(self, feedback: PasswordFeedback) -> ConfigBuilder {
        ConfigBuilder {
            feedback,
            ..self
        }
    }

    /// Sets the path to the TTY device.
    ///
    /// This can also be used to pass a temporary file for testing.
    pub fn input_path(self, path: String) -> ConfigBuilder {
        ConfigBuilder {
            input_path: Some(path),
            ..self
        }
    }

    /// Builds the final [`Config`].
    pub fn build(self) -> Config {
        Config {
            feedback: self.feedback,
            input_path: self.input_path,
        }
    }
}

#[cfg(not(target_family = "wasm"))]
struct FeedbackState {
    password: SafeString,
    displayed_count: usize,
    feedback: PasswordFeedback,
}

#[cfg(not(target_family = "wasm"))]
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

#[cfg(not(target_family = "wasm"))]
fn char_to_bytes(c: char) -> Vec<u8> {
    let mut buf = [0u8; 4];
    c.encode_utf8(&mut buf).as_bytes().to_vec()
}

#[cfg(target_family = "wasm")]
mod wasm {
    use super::{Config, ConfigBuilder, PasswordFeedback, SafeString};
    use std::io::{self, BufRead};

    /// Reads a password from the TTY
    pub fn read_password() -> std::io::Result<String> {
        read_password_with_config(ConfigBuilder::new().build())
    }

    /// Reads a password from TTY using the given config
    pub fn read_password_with_config(config: Config) -> std::io::Result<String> {
        let tty_path = config.input_path.as_deref().unwrap_or("/dev/tty");
        let tty = std::fs::File::open(tty_path)?;
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
}

#[cfg(all(target_family = "unix", not(target_family = "wasm")))]
mod unix {
    use libc::{c_int, isatty, tcsetattr, termios, ECHO, ECHONL, ICANON, ISIG, TCSANOW, VMIN, VTIME};
    use super::{Config, FeedbackState, PasswordFeedback};
    use std::fs::File;
    use std::io::{self, Write};
    use std::mem;
    use std::os::unix::io::AsRawFd;

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
        unsafe {
            isatty(fd) != 0
        }
    }

    fn safe_tcgetattr(fd: c_int) -> std::io::Result<termios> {
        let mut term = mem::MaybeUninit::<termios>::uninit();
        io_result(unsafe { ::libc::tcgetattr(fd, term.as_mut_ptr()) })?;
        Ok(unsafe { term.assume_init() })
    }

    fn safe_tcsetattr(fd: c_int, term: &mut termios) -> std::io::Result<()> {
        io_result(unsafe {tcsetattr(fd, TCSANOW, term)})
    }

    struct RawModeInput {
        tty: File,
        fd: i32,
        term_orig: Option<termios>,
        needs_terminal_configuration: bool,
        password_feedback: PasswordFeedback,
    }

    impl Drop for RawModeInput {
        fn drop(&mut self) {
            if let Some(ref mut term_orig) = self.term_orig {
                unsafe {
                    tcsetattr(self.fd, TCSANOW, term_orig);
                }
            }
        }
    }

    impl RawModeInput {
        fn new(config: Config) -> io::Result<RawModeInput> {
            let tty_path = config.input_path.as_deref().unwrap_or("/dev/tty");
            let tty = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(tty_path)?;
            let fd = tty.as_raw_fd();
            let is_a_tty = is_interactive_terminal(fd);
            Ok(RawModeInput {
                tty,
                fd,
                term_orig: if is_a_tty { Some(safe_tcgetattr(fd)?) } else { None },
                needs_terminal_configuration: is_a_tty,
                password_feedback: config.feedback,
            })
        }

        fn apply_terminal_configuration(&mut self) -> io::Result<()> {
            if !self.needs_terminal_configuration {
                panic!("apply_terminal_configuration called on non-TTY");
            }

            let mut term = safe_tcgetattr(self.fd)?;
            term.c_lflag &= !(ECHO | ICANON | ECHONL | ISIG);
            term.c_cc[VMIN] = 1;
            term.c_cc[VTIME] = 0;
            safe_tcsetattr(self.fd, &mut term)
        }

        fn read_password(&mut self) -> std::io::Result<String> {
            if self.needs_terminal_configuration {
                self.apply_terminal_configuration()?;
            }

            let mut state = FeedbackState::new(self.password_feedback);
            let mut byte = [0u8; 1];

            loop {
                let n = unsafe { libc::read(self.fd, byte.as_mut_ptr() as *mut libc::c_void, 1) };
                if n <= 0 {
                    return Err(std::io::Error::last_os_error());
                }

                match byte[0] {
                    // LF / CR (Enter)
                    b'\n' | b'\r' => {
                        self.tty.write_all(b"\n")?;
                        self.tty.flush()?;
                        break;
                    }
                    // Backspace / DEL
                    DEL | BACKSPACE => {
                        let output = state.pop_char();
                        if !output.is_empty() {
                            self.tty.write_all(&output)?;
                            self.tty.flush()?;
                        }
                    }
                    // Ctrl-U: clear line
                    CTRL_U => {
                        let output = state.clear();
                        if !output.is_empty() {
                            self.tty.write_all(&output)?;
                            self.tty.flush()?;
                        }
                    }
                    // Ctrl-C: interrupt
                    CTRL_C => {
                        self.tty.write_all(b"\n")?;
                        self.tty.flush()?;
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
                        let n = unsafe {
                            libc::read(self.fd, byte.as_mut_ptr() as *mut libc::c_void, 1)
                        };
                        if n > 0 && (byte[0] == b'[' || byte[0] == b'O') {
                            // CSI (ESC [) or SS3 (ESC O): read until final byte (0x40-0x7E)
                            loop {
                                let n = unsafe {
                                    libc::read(self.fd, byte.as_mut_ptr() as *mut libc::c_void, 1)
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
                            self.tty.write_all(&output)?;
                            self.tty.flush()?;
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
                            let n = unsafe {
                                libc::read(self.fd, byte.as_mut_ptr() as *mut libc::c_void, 1)
                            };
                            if n <= 0 {
                                break;
                            }
                            utf8_buf.push(byte[0]);
                        }
                        if let Ok(s) = std::str::from_utf8(&utf8_buf) {
                            if let Some(c) = s.chars().next() {
                                let output = state.push_char(c);
                                if !output.is_empty() {
                                    self.tty.write_all(&output)?;
                                    self.tty.flush()?;
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
}

#[cfg(target_family = "windows")]
mod windows {
    use super::{Config, FeedbackState, PasswordFeedback};
    use std::io::{self, Write};
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::{
        GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, ReadFile, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, ReadConsoleW, SetConsoleMode, CONSOLE_MODE,
        ENABLE_PROCESSED_INPUT,
    };

    const BACKSPACE: char = '\x08';
    const DEL: char = '\x7F';
    const CTRL_C: char = '\x03';
    const CTRL_D: char = '\x04';
    const CTRL_U: char = '\x15';
    const ESC: char = '\x1B';

    fn is_interactive_terminal(handle: windows_sys::Win32::Foundation::HANDLE) -> bool {
        let mut mode: CONSOLE_MODE = 0;
        unsafe {
            // Try to get the console mode. If it succeeds, the handle is a console handle.
            GetConsoleMode(handle, &mut mode) != 0
        }
    }

    fn get_console_mode(handle: HANDLE) -> io::Result<u32> {
        let mut mode: CONSOLE_MODE = 0;
        if unsafe { GetConsoleMode(handle, &mut mode as *mut CONSOLE_MODE) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(mode)
    }

    fn read_utf16_or_ascii_from_file(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<(u16, u32)> {
        let mut buf_bytes1: [u8; 1] = [0];
        let mut bytes_read1: u32 = 0;

        unsafe {
            if ReadFile(
                handle,
                buf_bytes1.as_mut_ptr() as *mut u8,
                buf_bytes1.len() as u32,
                &mut bytes_read1,
                std::ptr::null_mut(),
            ) == 0 {
                return Err(io::Error::last_os_error());
            }
        }

        // If no bytes were read, return None (EOF)
        if bytes_read1 == 0 {
            return Ok((0, bytes_read1));
        }

        // If the byte is ASCII (0x00-0x7F), return it as a u16
        if buf_bytes1[0] <= 0x7F {
            return Ok((buf_bytes1[0] as u16, bytes_read1));
        }

        // If the byte is the first byte of a UTF-16 character (0xC0-0xFF), read the next byte
        if buf_bytes1[0] >= 0xC0 {
            let mut buf_bytes2: [u8; 1] = [0];
            let mut bytes_read2: u32 = 0;

            unsafe {
                if ReadFile(
                    handle,
                    buf_bytes2.as_mut_ptr() as *mut u8,
                    buf_bytes2.len() as u32,
                    &mut bytes_read2,
                    std::ptr::null_mut(),
                ) == 0 {
                    return Err(io::Error::last_os_error());
                }
            }

            if bytes_read2 == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Incomplete UTF-16 character",
                ));
            }

            // Combine the two bytes into a u16
            let utf16_char = u16::from_le_bytes([buf_bytes1[0], buf_bytes2[0]]);
            return Ok((utf16_char, bytes_read1 + bytes_read2));
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid UTF-16 or ASCII character",
        ))
    }

    fn read_utf16_char_from_handle(handle: HANDLE, is_a_tty: bool) -> io::Result<(u16,u32)> {
        if is_a_tty {
            let mut buf: [u16; 1] = [0];
            let mut chars_read: u32 = 0;
            if unsafe {
                ReadConsoleW(
                    handle,
                    buf.as_mut_ptr() as *mut std::ffi::c_void,
                    1,
                    &mut chars_read,
                    std::ptr::null(),
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            return Ok((buf[0], chars_read));
        }

        read_utf16_or_ascii_from_file(handle)
    }

    struct RawModeInput {
        input_handle: HANDLE,
        output_handle: HANDLE,
        input_mode: u32,
        output_mode: u32,
        needs_terminal_configuration: bool,
        password_feedback: PasswordFeedback,
    }

    impl Drop for RawModeInput {
        fn drop(&mut self) {
            unsafe {
                SetConsoleMode(self.input_handle, self.input_mode);
            }
            unsafe {
                SetConsoleMode(self.output_handle, self.output_mode);
            }
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.input_handle);
            }
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.output_handle);
            }
        }
    }

    impl RawModeInput {
        fn new(config: Config) -> io::Result<RawModeInput> {
            let path_wide: Option<Vec<u16>> = config.input_path
                .map(|p| p.encode_utf16().chain(std::iter::once(0)).collect());

            let input_handle = unsafe {
                CreateFileW(
                    path_wide.clone()
                        .unwrap_or("CONIN$".encode_utf16().chain(std::iter::once(0)).collect())
                        .as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    INVALID_HANDLE_VALUE,
                )
            };
            if input_handle == INVALID_HANDLE_VALUE {
                return Err(std::io::Error::last_os_error());
            }

            let output_handle = unsafe {
                CreateFileW(
                    path_wide.clone()
                        .unwrap_or("CONOUT$".encode_utf16().chain(std::iter::once(0)).collect())
                        .as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    INVALID_HANDLE_VALUE,
                )
            };
            if output_handle == INVALID_HANDLE_VALUE {
                return Err(std::io::Error::last_os_error());
            }

            let is_a_tty = is_interactive_terminal(input_handle);

            Ok(RawModeInput {
                input_handle,
                output_handle,
                input_mode: if is_a_tty {
                    get_console_mode(input_handle)?
                } else { 0 },
                output_mode: if is_a_tty {
                    get_console_mode(output_handle)?
                } else { 0 },
                needs_terminal_configuration: is_a_tty,
                password_feedback: config.feedback,
            })
        }

        fn apply_terminal_configuration(&mut self) -> io::Result<()> {
            if !self.needs_terminal_configuration {
                panic!("apply_terminal_configuration called on non-TTY");
            }

            if unsafe { SetConsoleMode(self.input_handle, ENABLE_PROCESSED_INPUT) } == 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        }

        /// Reads a password from TTY using the given config
        pub fn read_password(&mut self) -> std::io::Result<String> {
            if self.needs_terminal_configuration {
                self.apply_terminal_configuration()?;
            }

            let mut out_stream = unsafe { std::fs::File::from_raw_handle(self.output_handle as _) };

            let mut state = FeedbackState::new(self.password_feedback);

            loop {
                let (wchar, chars_read) = read_utf16_char_from_handle(self.input_handle, self.needs_terminal_configuration)?;
                if chars_read == 0 {
                    continue;
                }

                // Handle UTF-16 surrogate pairs: characters above U+FFFF (e.g. emoji)
                // are split across two u16 values — a high surrogate (0xD800..0xDBFF)
                // followed by a low surrogate. Read the second half before decoding.
                let c = if (0xD800..=0xDBFF).contains(&wchar) {
                    let (wchar2, chars_read2) = read_utf16_char_from_handle(self.input_handle, self.needs_terminal_configuration)?;
                    // TODO: check chars_read2 == 0, don't try to decode if it is
                    match char::decode_utf16([wchar, wchar2])
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
                        match read_utf16_char_from_handle(self.input_handle, self.needs_terminal_configuration) {
                            Ok((wchar3, chars_read3)) => {
                                // TODO: check chars_read3 == 0, don't try to decode if it is
                                if wchar3 == b'[' as u16 || wchar3 == b'O' as u16 {
                                    // CSI (ESC [) or SS3 (ESC O): read until final byte (0x40-0x7E)
                                    loop {
                                        let mut buf4: [u16; 1] = [0];
                                        let mut chars_read4: u32 = 0;
                                        if unsafe {
                                            ReadConsoleW(
                                                self.input_handle,
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
                            Err(_) => {
                                // TODO: Handle errors?
                            }
                        }
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

    /// Reads a password from TTY using the given config
    pub fn read_password_with_config(config: Config) -> std::io::Result<String> {
        let mut raw_mode_input = RawModeInput::new(config)?;
        raw_mode_input.read_password()
    }

    /// Reads a password from the TTY
    pub fn read_password() -> std::io::Result<String> {
        read_password_with_config(Config::default())
    }
}

#[cfg(all(target_family = "unix", not(target_family = "wasm")))]
pub use unix::read_password;
#[cfg(all(target_family = "unix", not(target_family = "wasm")))]
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

    #[cfg(not(target_family = "wasm"))]
    mod feedback_state_tests {
        use crate::{FeedbackState, PasswordFeedback};

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

    #[cfg(all(target_family = "unix", not(target_family = "wasm")))]
    mod unix {
        use crate::{read_password_with_config, ConfigBuilder};
        use std::io::Write;

        #[test]
        fn test_read_password_with_config() {
            let mut temp_file = tempfile::NamedTempFile::new().unwrap();
            temp_file.write_all(b"password\n").unwrap();
            let path = temp_file.path().to_str().unwrap().to_string();

            let config = ConfigBuilder::new()
                .input_path(path)
                .build();

            // This should fail because it's not a TTY (tcgetattr fails on regular files)
            // But it proves that read_password_with_config is using our input path.
            let result = read_password_with_config(config);
            assert_eq!("password", result.unwrap());
        }

        #[test]
        fn test_read_password_with_config_errors_with_file_not_found() {
            let config = ConfigBuilder::new()
                .input_path("/does/not/exist".to_string())
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

    #[cfg(target_family = "windows")]
    mod windows {
        use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND};
        use crate::{read_password_with_config, ConfigBuilder};
        use std::io::Write;

        #[test]
        fn test_read_password_with_config() {
            let mut temp_file = tempfile::NamedTempFile::new().unwrap();
            temp_file.write_all(b"password\r\n").unwrap();
            let path = temp_file.path().to_str().unwrap().to_string();

            let config = ConfigBuilder::new()
                .input_path(path)
                .build();

            let result = read_password_with_config(config);
            assert_eq!("password", result.unwrap());
        }

        #[test]
        fn test_read_password_with_config_errors_with_file_not_found() {
            let config = ConfigBuilder::new()
                .input_path("C:\\not-found.txt".to_string())
                .build();

            // This should fail because it's not a Console (GetConsoleMode fails on regular files)
            // But, it proves that read_password_with_config is using our input path.
            let result = read_password_with_config(config);
            assert!(result.is_err());

            let err = result.unwrap_err();
            assert_eq!(err.raw_os_error(), Some(ERROR_FILE_NOT_FOUND as i32));
        }
    }
}
