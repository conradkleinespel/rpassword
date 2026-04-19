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
//! For testing or custom use-cases, you can use `read_password_with_config` and `prompt_password_with_config`:
//! ```
//! use tempfile::NamedTempFile;
//! use std::io::Write;
//! use rpassword::{PasswordFeedback, InputOutput};
//!
//! let mut input = NamedTempFile::new().unwrap();
//! input.write_all(b"my-password\n").unwrap();
//!
//! let mut output = NamedTempFile::new().unwrap();
//!
//! let config = rpassword::ConfigBuilder::new()
//!     // Default input/output is the console, but we can pass any file path
//!     .input_output(InputOutput::InputOutput(
//!         input.path().to_str().unwrap().to_string(),
//!         output.path().to_str().unwrap().to_string(),
//!     ))
//!     // Default behavior is to hide the password as it's being typed, but we can change that
//!     .password_feedback(PasswordFeedback::Mask('*'))
//!     .build();
//!
//! let password = rpassword::read_password_with_config(config).unwrap();
//! println!("Your password is {}", password);
//! ```

use rtoolbox::fix_line_issues::fix_line_issues;
use rtoolbox::print_tty::{print_writer};
use rtoolbox::safe_string::SafeString;
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::{BufRead, Write};

#[cfg(windows)]
mod defaults {
    pub const DEFAULT_INPUT_PATH: &str = "CONIN$";
    pub const DEFAULT_OUTPUT_PATH: &str = "CONOUT$";
}

#[cfg(any(all(target_family = "unix", not(target_family = "wasm")), target_family = "wasm"))]
mod defaults {
    pub const DEFAULT_INPUT_PATH: &str = "/dev/tty";
    pub const DEFAULT_OUTPUT_PATH: &str = "/dev/tty";
}

/// Controls visual feedback when the user types a password.
///
/// # Examples
///
/// ## Using `PasswordFeedback::Mask` to show asterisks (`*`) while typing:
/// ```
/// use rpassword::{ConfigBuilder, PasswordFeedback};
///
/// let config = ConfigBuilder::new()
///     .password_feedback(PasswordFeedback::Mask('*'))
///     .build();
/// ```
///
/// ## Using `PasswordFeedback::PartialMask` to show the first 3 characters in plaintext, then asterisks (`*`):
/// ```
/// use rpassword::{ConfigBuilder, PasswordFeedback};
///
/// let config = ConfigBuilder::new()
///     .password_feedback(PasswordFeedback::PartialMask('*', 3))
///     .build();
/// ```
///
/// ## Using `PasswordFeedback::Hide` (default behavior):
/// ```
/// use rpassword::{ConfigBuilder, PasswordFeedback};
///
/// let config = ConfigBuilder::new()
///     .password_feedback(PasswordFeedback::Hide)
///     .build();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PasswordFeedback {
    /// Show nothing while typing (current default behavior).
    #[default]
    Hide,
    /// Show the given mask char for every character typed.
    /// e.g. `Mask('*')` shows stars.
    Mask(char),
    /// Show the actual character for the first N chars, then the given
    /// mask char for the rest.
    /// e.g. `PartialMask('*', 3)` shows first 3 chars in plaintext, then stars.
    PartialMask(char, usize),
}

/// Configuration for prompting and reading a password.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    pub(crate) feedback: PasswordFeedback,
    pub(crate) input_output: Option<InputOutput>,
}

/// A builder for creating a [`Config`].
///
/// This struct provides a convenient way to configure the behavior of password reading,
/// such as setting visual feedback and specifying an input path.
///
/// # Examples
///
/// ## Basic Usage
/// ```
/// use rpassword::{ConfigBuilder, PasswordFeedback};
///
/// let config = ConfigBuilder::new()
///     .password_feedback(PasswordFeedback::Mask('*'))
///     .build();
/// ```
///
/// ## Setting Custom Input/Output Paths
/// ```
/// use rpassword::{ConfigBuilder, InputOutput};
///
/// let config = ConfigBuilder::new()
///     .input_output(InputOutput::InputOutputCombined("/dev/tty".to_string()))
///     .build();
/// ```
///
/// ## Combining Feedback and Input/Output Paths
/// ```
/// use rpassword::{ConfigBuilder, PasswordFeedback, InputOutput};
///
/// let config = ConfigBuilder::new()
///     .password_feedback(PasswordFeedback::PartialMask('*', 3))
///     .input_output(InputOutput::InputOutputCombined("/dev/tty".to_string()))
///     .build();
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigBuilder {
    feedback: PasswordFeedback,
    input_output: Option<InputOutput>,
}

/// Configuration for customizing input and output streams or paths.
///
/// This enum allows you to specify custom input and output streams or a path that applies to both.
/// It is useful for testing or scenarios where you need to override the default behavior.
///
/// The default behavior is to use the console for input and output, in a cross-platform way.
///
/// # Examples
///
/// ## Setting a Custom Input Path
/// ```
/// use rpassword::{ConfigBuilder, InputOutput};
///
/// let config = ConfigBuilder::new()
///     .input_output(InputOutput::Input("/dev/tty".to_string()))
///     .build();
/// ```
///
/// ## Setting a Custom Output Path
/// ```
/// use rpassword::{ConfigBuilder, InputOutput};
///
/// let config = ConfigBuilder::new()
///     .input_output(InputOutput::Output("/dev/tty".to_string()))
///     .build();
/// ```
///
/// ## Setting Both Custom Input and Output Paths
/// ```
/// use rpassword::{ConfigBuilder, InputOutput};
///
/// let config = ConfigBuilder::new()
///     .input_output(InputOutput::InputOutput(
///         "/dev/tty".to_string(),
///         "/dev/tty".to_string()
///     ))
///     .build();
/// ```
///
/// ## Setting a Combined Path for Both Input and Output
/// ```
/// use rpassword::{ConfigBuilder, InputOutput};
///
/// let config = ConfigBuilder::new()
///     .input_output(InputOutput::InputOutputCombined("/dev/tty".to_string()))
///     .build();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputOutput {
    Input(String),
    Output(String),
    InputOutputCombined(String),
    InputOutput(String, String),
}

impl InputOutput {
    fn get_input_path(&self) -> Option<&str> {
        match self {
            InputOutput::Input(path) => Some(path.as_str()),
            InputOutput::InputOutput(input_path, _) => Some(input_path.as_str()),
            InputOutput::InputOutputCombined(path) => Some(path.as_str()),
            _ => None,
        }
    }

    fn get_output_path(&self) -> Option<&str> {
        match self {
            InputOutput::Output(path) => Some(path.as_str()),
            InputOutput::InputOutput(_, output_path) => Some(output_path.as_str()),
            InputOutput::InputOutputCombined(path) => Some(path.as_str()),
            _ => None,
        }
    }
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

    /// Sets the path to the input and output files (defaults to the console).
    ///
    /// This can also be used to pass a temporary file for testing.
    pub fn input_output(self, input_output: InputOutput) -> ConfigBuilder {
        ConfigBuilder {
            input_output: Some(input_output),
            ..self
        }
    }

    /// Builds the final [`Config`].
    pub fn build(self) -> Config {
        Config {
            feedback: self.feedback,
            input_output: self.input_output,
        }
    }
}

#[cfg(any(all(target_family = "unix", not(target_family = "wasm")), target_family = "windows"))]
struct FeedbackState {
    password: SafeString,
    needs_terminal_configuration: bool,
    displayed_count: usize,
    feedback: PasswordFeedback,
}

#[cfg(any(all(target_family = "unix", not(target_family = "wasm")), target_family = "windows"))]
impl FeedbackState {
    fn new(feedback: PasswordFeedback, needs_terminal_configuration: bool) -> Self {
        FeedbackState {
            password: SafeString::new(),
            needs_terminal_configuration,
            displayed_count: 0,
            feedback,
        }
    }

    fn push_char(&mut self, c: char) -> Vec<u8> {
        self.password.push(c);

        if !self.needs_terminal_configuration {
            return Vec::new();
        }

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

            if !self.needs_terminal_configuration {
                return Vec::new();
            }

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
        self.password = SafeString::new();

        if !self.needs_terminal_configuration {
            return Vec::new();
        }

        let count = self.displayed_count;
        self.displayed_count = 0;
        [0x08u8, b' ', 0x08].repeat(count)
    }

    fn abort(&mut self) -> Vec<u8> {
        self.password = SafeString::new();

        if !self.needs_terminal_configuration {
            return Vec::new();
        }

        self.displayed_count = 0;
        [b'\n'].to_vec()
    }

    fn finish(&mut self) -> Vec<u8> {
        if !self.needs_terminal_configuration {
            return Vec::new();
        }

        [b'\n'].to_vec()
    }

    fn is_empty(&self) -> bool {
        self.password.is_empty()
    }

    fn into_password(self) -> String {
        self.password.into_inner()
    }
}

#[cfg(any(all(target_family = "unix", not(target_family = "wasm")), target_family = "windows"))]
fn char_to_bytes(c: char) -> Vec<u8> {
    let mut buf = [0u8; 4];
    c.encode_utf8(&mut buf).as_bytes().to_vec()
}

#[cfg(target_family = "wasm")]
mod wasm {
    use super::{Config, ConfigBuilder, PasswordFeedback, SafeString};
    use std::io::{self, BufRead};
    use crate::defaults::DEFAULT_INPUT_PATH;

    /// Reads a password from the TTY
    pub fn read_password() -> std::io::Result<String> {
        read_password_with_config(ConfigBuilder::new().build())
    }

    /// Reads a password from TTY using the given config
    pub fn read_password_with_config(config: Config) -> std::io::Result<String> {
        let tty_path = config.input_output.and_then(|p| p.get_input_path().map(|path| path.to_owned())).unwrap_or(DEFAULT_INPUT_PATH.to_string());
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
    use std::io::{self, Read, Write};
    use std::mem;
    use std::os::unix::io::AsRawFd;
    use crate::defaults::{DEFAULT_INPUT_PATH, DEFAULT_OUTPUT_PATH};

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
            let input_path = config.input_output.clone().and_then(|p| p.get_input_path().map(|path| path.to_owned())).unwrap_or(DEFAULT_INPUT_PATH.to_string());
            let input_file = std::fs::OpenOptions::new()
                .read(true)
                .open(input_path)?;
            let input_fd = input_file.as_raw_fd();
            let is_a_tty = is_interactive_terminal(input_fd);

            let output_path = config.input_output.clone().and_then(|p| p.get_output_path().map(|path| path.to_owned())).unwrap_or(DEFAULT_OUTPUT_PATH.to_string());
            let output_file = std::fs::OpenOptions::new()
                .write(true)
                .open(output_path)?;

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
    use crate::defaults::{DEFAULT_INPUT_PATH, DEFAULT_OUTPUT_PATH};

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

    fn open_file(path: &str) -> io::Result<HANDLE> {
        let handle = unsafe {
            CreateFileW(
                path.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>().as_ptr(),
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

        Ok(handle)
    }

    fn read_single_byte_from_file(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<(u8, u32)> {
        let mut buf_bytes1: [u8; 1] = [0];
        let mut bytes_read1: u32 = 0;

        unsafe {
            if ReadFile(
                handle,
                buf_bytes1.as_mut_ptr(),
                buf_bytes1.len() as u32,
                &mut bytes_read1,
                std::ptr::null_mut(),
            ) == 0 {
                return Err(io::Error::last_os_error());
            }
        }

        if bytes_read1 == 0 {
            return Ok((0, 0))
        }

        return Ok((buf_bytes1[0], bytes_read1));
    }

    fn read_single_char_from_console(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<(u16, u32)> {
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

    /// Read an UTF-16 char from a file
    fn read_utf16_or_ascii_from_file(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<(u16, u32)> {
        let (byte1, bytes_read1) = read_single_byte_from_file(handle)?;

        // If the byte is ASCII (0x00-0x7F), return it as a u16
        if byte1 <= 0x7F {
            return Ok((byte1 as u16, bytes_read1));
        }

        // If the byte is the first byte of a UTF-16 character (0xC0-0xFF), read the next byte
        if byte1 >= 0xC0 {
            let (byte2, bytes_read2) = read_single_byte_from_file(handle)?;
            if bytes_read2 == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Incomplete UTF-16 character",
                ));
            }

            // Combine the two bytes into a u16
            let utf16_char = u16::from_le_bytes([byte1, byte2]);
            return Ok((utf16_char, bytes_read1 + bytes_read2));
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid UTF-16 or ASCII character",
        ))
    }

    fn read_utf16_char_from_handle(handle: HANDLE, is_a_tty: bool) -> io::Result<(u16,u32)> {
        if is_a_tty {
            read_single_char_from_console(handle)
        } else {
            read_utf16_or_ascii_from_file(handle)
        }
    }

    #[derive(Debug)]
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
            let same_input_output = self.input_handle == self.output_handle;

            unsafe {
                SetConsoleMode(self.input_handle, self.input_mode);
            }
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.input_handle);
            }

            if same_input_output {
                return;
            }

            unsafe {
                SetConsoleMode(self.output_handle, self.output_mode);
            }
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.output_handle);
            }
        }
    }

    impl RawModeInput {
        fn new(config: Config) -> io::Result<RawModeInput> {
            let input_path_wide: Vec<u16> = config.input_output.clone().and_then(|p| p.get_input_path().map(|path| path.to_owned()))
                .unwrap_or(DEFAULT_INPUT_PATH.to_string()).encode_utf16().chain(std::iter::once(0)).collect();
            let output_path_wide: Vec<u16> = config.input_output.clone().and_then(|p| p.get_output_path().map(|path| path.to_owned()))
                .unwrap_or(DEFAULT_OUTPUT_PATH.to_string()).encode_utf16().chain(std::iter::once(0)).collect();

            let input_handle = open_file(
                match config.input_output {
                    Some(ref v) => v.get_input_path().unwrap_or(DEFAULT_INPUT_PATH),
                    _ => DEFAULT_INPUT_PATH,
                }
            )?;

            let output_handle = if input_path_wide == output_path_wide {
                input_handle
            } else {
                let output_handle = open_file(
                    match config.input_output {
                        Some(ref v) => v.get_output_path().unwrap_or(DEFAULT_OUTPUT_PATH),
                        _ => DEFAULT_OUTPUT_PATH,
                    }
                )?;
                output_handle
            };

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

            let mut state = FeedbackState::new(self.password_feedback, self.needs_terminal_configuration);

            loop {
                let (wchar, chars_read) = read_utf16_char_from_handle(self.input_handle, self.needs_terminal_configuration)?;
                if chars_read == 0 {
                    // EOF
                    break;
                }

                // Handle UTF-16 surrogate pairs: characters above U+FFFF (e.g. emoji)
                // are split across two u16 values — a high surrogate (0xD800..0xDBFF)
                // followed by a low surrogate. Read the second half before decoding.
                let c = if (0xD800..=0xDBFF).contains(&wchar) {
                    let (wchar2, chars_read2) = read_utf16_char_from_handle(self.input_handle, self.needs_terminal_configuration)?;
                    if chars_read2 == 0 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Incomplete UTF-16 character",
                        ));
                    }
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
                        let output = state.finish();
                        if !output.is_empty() {
                            out_stream.write_all(&output)?;
                            out_stream.flush()?;
                        }
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
                        let output = state.abort();
                        if !output.is_empty() {
                            out_stream.write_all(&output)?;
                            out_stream.flush()?;
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
                        let (wchar3, chars_read3) = read_utf16_char_from_handle(self.input_handle, self.needs_terminal_configuration)?;
                        if chars_read3 == 0 {
                            // EOF, ignore the incomplete escape sequence
                            break;
                        }

                        if wchar3 == b'[' as u16 || wchar3 == b'O' as u16 {
                            // CSI (ESC [) or SS3 (ESC O): read until final byte (0x40-0x7E)
                            loop {
                                let (wchar4, chars_read4) = read_utf16_char_from_handle(self.input_handle, self.needs_terminal_configuration)?;
                                if chars_read4 == 0 {
                                    // EOF, ignore the incomplete escape sequence
                                    break;
                                }
                                if (0x40..=0x7E).contains(&wchar4) {
                                    break;
                                }
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
use crate::defaults::{DEFAULT_OUTPUT_PATH};

/// Reads a password from `impl BufRead`.
///
/// **Deprecated**: This method is deprecated. Use `read_password_with_config` with a temporary file instead.
/// See the example below for updated usage.
///
/// # Example of Updated Usage
/// ```
/// use tempfile::NamedTempFile;
/// use std::io::Write;
/// use rpassword::{ConfigBuilder, InputOutput, read_password_with_config};
///
/// let mut input = NamedTempFile::new().unwrap();
/// input.write_all(b"my-password\n").unwrap();
///
/// let config = ConfigBuilder::new()
///     .input_output(InputOutput::InputOutputCombined(
///         input.path().to_str().unwrap().to_string(),
///     ))
///     .build();
///
/// let password = read_password_with_config(config).unwrap();
/// println!("The typed password is: {}", password);
/// ```
#[deprecated(
    since = "7.5.0",
    note = "Use `read_password_with_config` with a temporary file instead. See the example above for updated usage."
)]
pub fn read_password_from_bufread(reader: &mut impl BufRead) -> std::io::Result<String> {
    let mut password = SafeString::new();
    reader.read_line(&mut password)?;

    fix_line_issues(password.into_inner())
}

/// Prompts on `impl Write` and then reads a password from `impl BufRead`.
///
/// **Deprecated**: This method is deprecated. Use `prompt_password_with_config` with a temporary file instead.
/// See the example below for updated usage.
///
/// # Example of Updated Usage
/// ```
/// use tempfile::NamedTempFile;
/// use std::io::Write;
/// use rpassword::{ConfigBuilder, InputOutput, prompt_password_with_config};
///
/// let mut input = NamedTempFile::new().unwrap();
/// input.write_all(b"my-password\n").unwrap();
///
/// let mut output = NamedTempFile::new().unwrap();
///
/// let config = ConfigBuilder::new()
///     .input_output(InputOutput::InputOutput(
///         input.path().to_str().unwrap().to_string(),
///         output.path().to_str().unwrap().to_string(),
///     ))
///     .build();
///
/// let password = prompt_password_with_config("Your password: ", config).unwrap();
/// println!("The typed password is: {}", password);
/// ```
#[deprecated(
    since = "7.5.0",
    note = "Use `prompt_password_with_config` with a temporary file instead. See the example above for updated usage."
)]
#[allow(deprecated)]
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
    prompt_password_with_config(prompt, ConfigBuilder::new().build())
}

/// Prompts and then reads a password using the given config
pub fn prompt_password_with_config(
    prompt: impl ToString,
    config: Config,
) -> std::io::Result<String> {
    match config.input_output {
        Some(ref io) => {
            let mut file = OpenOptions::new()
                .write(true)
                .open(
                    io.get_output_path().map(|path| path.to_owned())
                        .unwrap_or(DEFAULT_OUTPUT_PATH.to_string())
                )?;
            file.write_all(prompt.to_string().as_bytes())?;
            file.flush()?;
        }
        None => {
            let mut file = OpenOptions::new()
                .write(true)
                .open(DEFAULT_OUTPUT_PATH)?;
            file.write_all(prompt.to_string().as_bytes())?;
            file.flush()?;
        }
    };
    read_password_with_config(config)
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
    #[allow(deprecated)]
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

    #[cfg(any(all(target_family = "unix", not(target_family = "wasm")), target_family = "windows"))]
    mod feedback_state_tests {
        mod with_terminal_configuration {
            use crate::{FeedbackState, PasswordFeedback};

            #[test]
            fn feedback_state_mask_star() {
                let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), true);
                assert_eq!(state.push_char('a'), b"*");
                assert_eq!(state.push_char('b'), b"*");
                assert_eq!(state.push_char('c'), b"*");
                assert_eq!(state.pop_char(), vec![0x08, b' ', 0x08]);
                assert_eq!(state.into_password(), "ab");
            }

            #[test]
            fn feedback_state_mask_hash() {
                let mut state = FeedbackState::new(PasswordFeedback::Mask('#'), true);
                assert_eq!(state.push_char('x'), b"#");
                assert_eq!(state.push_char('y'), b"#");
                assert_eq!(state.into_password(), "xy");
            }

            #[test]
            fn feedback_state_hide() {
                let mut state = FeedbackState::new(PasswordFeedback::Hide, true);
                assert!(state.push_char('a').is_empty());
                assert!(state.push_char('b').is_empty());
                assert!(state.pop_char().is_empty());
                assert_eq!(state.into_password(), "a");
            }

            #[test]
            fn feedback_state_partial_mask() {
                let mut state = FeedbackState::new(PasswordFeedback::PartialMask('*', 3), true);
                assert_eq!(state.push_char('a'), b"a");
                assert_eq!(state.push_char('b'), b"b");
                assert_eq!(state.push_char('c'), b"c");
                assert_eq!(state.push_char('d'), b"*");
                assert_eq!(state.push_char('e'), b"*");
                assert_eq!(state.into_password(), "abcde");
            }

            #[test]
            fn feedback_state_backspace_empty() {
                let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), true);
                assert!(state.pop_char().is_empty());
            }

            #[test]
            fn feedback_state_clear() {
                let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), true);
                state.push_char('a');
                state.push_char('b');
                state.push_char('c');
                assert_eq!(state.clear(), [0x08u8, b' ', 0x08].repeat(3));
                assert!(state.is_empty());
            }

            #[test]
            fn feedback_state_abort() {
                let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), true);
                state.push_char('a');
                state.push_char('b');
                state.push_char('c');
                assert_eq!(state.abort(), [b'\n']);
                assert!(state.is_empty());
            }

            #[test]
            fn feedback_state_finish() {
                let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), true);
                state.push_char('a');
                state.push_char('b');
                state.push_char('c');
                assert_eq!(state.finish(), [b'\n']);
                assert_eq!(state.into_password(), "abc");
            }

            #[test]
            fn feedback_state_partial_mask_zero() {
                let mut state = FeedbackState::new(PasswordFeedback::PartialMask('*', 0), true);
                assert_eq!(state.push_char('a'), b"*");
                assert_eq!(state.push_char('b'), b"*");
                assert_eq!(state.into_password(), "ab");
            }
        }

        mod without_terminal_configuration {
            use crate::{FeedbackState, PasswordFeedback};

            #[test]
            fn feedback_state_mask_star() {
                let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), false);
                assert_eq!(state.push_char('a'), vec![]);
                assert_eq!(state.push_char('b'), vec![]);
                assert_eq!(state.push_char('c'), vec![]);
                assert_eq!(state.pop_char(), vec![]);
                assert_eq!(state.into_password(), "ab");
            }

            #[test]
            fn feedback_state_mask_hash() {
                let mut state = FeedbackState::new(PasswordFeedback::Mask('#'), false);
                assert_eq!(state.push_char('x'), vec![]);
                assert_eq!(state.push_char('y'), vec![]);
                assert_eq!(state.into_password(), "xy");
            }

            #[test]
            fn feedback_state_hide() {
                let mut state = FeedbackState::new(PasswordFeedback::Hide, false);
                assert!(state.push_char('a').is_empty());
                assert!(state.push_char('b').is_empty());
                assert!(state.pop_char().is_empty());
                assert_eq!(state.into_password(), "a");
            }

            #[test]
            fn feedback_state_partial_mask() {
                let mut state = FeedbackState::new(PasswordFeedback::PartialMask('*', 3), false);
                assert_eq!(state.push_char('a'), vec![]);
                assert_eq!(state.push_char('b'), vec![]);
                assert_eq!(state.push_char('c'), vec![]);
                assert_eq!(state.push_char('d'), vec![]);
                assert_eq!(state.push_char('e'), vec![]);
                assert_eq!(state.into_password(), "abcde");
            }

            #[test]
            fn feedback_state_backspace_empty() {
                let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), false);
                assert!(state.pop_char().is_empty());
            }

            #[test]
            fn feedback_state_clear() {
                let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), false);
                state.push_char('a');
                state.push_char('b');
                state.push_char('c');
                assert_eq!(state.clear(), vec![]);
                assert!(state.is_empty());
            }

            #[test]
            fn feedback_state_abort() {
                let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), false);
                state.push_char('a');
                state.push_char('b');
                state.push_char('c');
                assert_eq!(state.abort(), vec![]);
                assert!(state.is_empty());
            }

            #[test]
            fn feedback_state_finish() {
                let mut state = FeedbackState::new(PasswordFeedback::Mask('*'), false);
                state.push_char('a');
                state.push_char('b');
                state.push_char('c');
                assert_eq!(state.finish(), vec![]);
                assert_eq!(state.into_password(), "abc");
            }

            #[test]
            fn feedback_state_partial_mask_zero() {
                let mut state = FeedbackState::new(PasswordFeedback::PartialMask('*', 0), false);
                assert_eq!(state.push_char('a'), vec![]);
                assert_eq!(state.push_char('b'), vec![]);
                assert_eq!(state.into_password(), "ab");
            }
        }
    }

    #[cfg(all(target_family = "unix", not(target_family = "wasm")))]
    mod unix {
        use crate::{read_password_with_config, ConfigBuilder, InputOutput};
        use std::io::Write;

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

    #[cfg(target_family = "windows")]
    mod windows {
        use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND};
        use crate::{read_password_with_config, ConfigBuilder, InputOutput};
        use std::io::{Write};

        #[test]
        fn test_read_password_with_config() {
            let mut temp_file = tempfile::NamedTempFile::new().unwrap();
            temp_file.write_all(b"password\r\n").unwrap();
            let path = temp_file.path().to_str().unwrap().to_string();

            let config = ConfigBuilder::new()
                .input_output(InputOutput::InputOutputCombined(path.clone()))
                .build();

            let result = read_password_with_config(config);
            assert_eq!("password", result.unwrap());
        }

        #[test]
        fn test_read_password_with_config_errors_with_file_not_found() {
            let config = ConfigBuilder::new()
                .input_output(InputOutput::InputOutputCombined("C:\\not-found.txt".to_string()))
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
