use crate::config::{Config, PasswordFeedback};
use crate::feedback::FeedbackState;
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

pub const DEFAULT_INPUT_PATH: &str = "CONIN$";
pub const DEFAULT_OUTPUT_PATH: &str = "CONOUT$";

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
        let input_handle = open_file(config.input_path.as_str())?;
        let output_handle = if config.input_path == config.output_path {
            input_handle
        } else {
            let output_handle = open_file(config.output_path.as_str())?;
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

#[cfg(test)]
mod tests {
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