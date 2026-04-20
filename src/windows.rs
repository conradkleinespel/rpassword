use crate::config::{Config};
use std::io;
use windows_sys::Win32::Foundation::{
    GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{CreateFileW, ReadFile, WriteFile, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
use windows_sys::Win32::System::Console::{GenerateConsoleCtrlEvent, GetConsoleMode, ReadConsoleW, SetConsoleMode, WriteConsoleW, CONSOLE_MODE, CTRL_C_EVENT, ENABLE_PROCESSED_INPUT};
use crate::RawPasswordInput;

pub const DEFAULT_INPUT_PATH: &str = "CONIN$";
pub const DEFAULT_OUTPUT_PATH: &str = "CONOUT$";

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

fn read_wchar_from_console(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<u16> {
    let mut buf: [u16; 1] = [0];
    let mut wchars_read: u32 = 0;
    if unsafe {
        ReadConsoleW(
            handle,
            buf.as_mut_ptr() as *mut std::ffi::c_void,
            1,
            &mut wchars_read,
            std::ptr::null(),
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    if wchars_read == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "unexpected end of file",
        ));
    }

    Ok(buf[0])
}

fn read_char_from_console(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<char> {
    let wchar1 = read_wchar_from_console(handle)?;
    // Handle UTF-16 surrogate pairs: characters above U+FFFF (e.g. emoji)
    // are split across two u16 values — a high surrogate (0xD800..0xDBFF)
    // followed by a low surrogate. Read the second half before decoding.
    if (0xD800..=0xDBFF).contains(&wchar1) {
        let wchar2 = match read_wchar_from_console(handle) {
            Ok(wchar) => wchar,
            Err(e) => {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    return Ok('\u{FFFD}');
                }
                return Err(e);
            },
        };
        match char::decode_utf16([wchar1, wchar2])
            .next()
            .and_then(|r| r.ok())
        {
            Some(c) => Ok(c),
            // Invalid/mismatched surrogate pair; shouldn't happen with
            // ReadConsoleW, but we skip gracefully rather than panicking.
            None => Ok('\u{FFFD}'),
        }
    } else {
        match char::from_u32(wchar1 as u32) {
            Some(c) => Ok(c),
            // Orphaned surrogate (0xD800-0xDFFF) as a lone u16; defensive
            // guard since ReadConsoleW shouldn't produce these.
            None => Ok('\u{FFFD}'),
        }
    }
}

fn read_byte_from_file(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<u8> {
    let mut buf_bytes: [u8; 1] = [0];
    let mut bytes_read: u32 = 0;

    unsafe {
        if ReadFile(
            handle,
            buf_bytes.as_mut_ptr(),
            buf_bytes.len() as u32,
            &mut bytes_read,
            std::ptr::null_mut(),
        ) == 0 {
            return Err(io::Error::last_os_error());
        }
    }

    if bytes_read == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "unexpected end of file",
        ));
    }

    Ok(buf_bytes[0])
}

fn read_char_from_file(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<char> {
    let byte1 = read_byte_from_file(handle)?;
    match byte1 {
        // ASCII
        0x00..=0x7F => Ok(byte1 as char),
        // UTF-8 lead byte
        0xC0..=0xF7 => {
            let width = match byte1 {
                0xC0..=0xDF => 2,
                0xE0..=0xEF => 3,
                0xF0..=0xF7 => 4,
                _ => unreachable!(),
            };
            let mut utf8_buf = vec![byte1];
            for _ in 1..width {
                match read_byte_from_file(handle) {
                    Ok(next_byte) => {
                        utf8_buf.push(next_byte);
                    },
                    Err(e) => {
                        if e.kind() == io::ErrorKind::UnexpectedEof {
                            return Ok('\u{FFFD}');
                        }
                        return Err(e);
                    }
                }
            }
            match std::str::from_utf8(&utf8_buf) {
                Ok(s) => {
                    if let Some(c) = s.chars().next() {
                        Ok(c)
                    } else {
                        Ok('\u{FFFD}')
                    }
                }
                _ => Ok('\u{FFFD}'),
            }
        }
        // Invalid byte
        _ => Ok('\u{FFFD}'),
    }
}

fn write_output_to_console(handle: windows_sys::Win32::Foundation::HANDLE, output: &str) -> std::io::Result<()> {
    let output_utf16 = output.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let mut wchars_written: u32 = 0;
    unsafe {
        if WriteConsoleW(
            handle,
            output_utf16.as_ptr(),
            output_utf16.len() as u32,
            &mut wchars_written,
            std::ptr::null_mut(),
        ) == 0 {
            return Err(std::io::Error::last_os_error());
        }
    }

    Ok(())
}

fn write_output_to_file(handle: windows_sys::Win32::Foundation::HANDLE, output: &str) -> std::io::Result<()> {
    let output_bytes = output.as_bytes();
    let mut bytes_written: u32 = 0;
    unsafe {
        if WriteFile(
            handle,
            output_bytes.as_ptr(),
            output_bytes.len() as u32,
            &mut bytes_written,
            std::ptr::null_mut(),
        ) == 0 {
            return Err(std::io::Error::last_os_error());
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct RawModeInput {
    input_handle: HANDLE,
    output_handle: HANDLE,
    input_mode: u32,
    output_mode: u32,
    needs_terminal_configuration: bool,
}

impl Drop for RawModeInput {
    fn drop(&mut self) {
        let same_input_output = self.input_handle == self.output_handle;

        if self.input_handle != INVALID_HANDLE_VALUE {
            unsafe {
                SetConsoleMode(self.input_handle, self.input_mode);
            }
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.input_handle);
            }
        }

        if same_input_output {
            return;
        }

        if self.output_handle != INVALID_HANDLE_VALUE {
            unsafe {
                SetConsoleMode(self.output_handle, self.output_mode);
            }
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.output_handle);
            }
        }
    }
}

impl RawPasswordInput for RawModeInput {
    fn new(config: Config) -> io::Result<impl RawPasswordInput> {
        let input_handle = open_file(config.input_path.as_str())?;
        let output_handle = if config.input_path == config.output_path {
            input_handle
        } else {
            open_file(config.output_path.as_str())?
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
        })
    }

    fn needs_terminal_configuration(&self) -> bool {
        self.needs_terminal_configuration
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

    fn read_char(&mut self) -> io::Result<char> {
        if self.needs_terminal_configuration {
            read_char_from_console(self.input_handle)
        } else {
            read_char_from_file(self.input_handle)
        }
    }

    fn write_output(&mut self, output: &str) -> std::io::Result<()> {
        if self.needs_terminal_configuration {
            write_output_to_console(self.output_handle, output)
        } else {
            write_output_to_file(self.output_handle, output)
        }
    }

    fn send_signal_sigint(&mut self) -> io::Result<()> {
        if unsafe {
            GenerateConsoleCtrlEvent(CTRL_C_EVENT, 0)
        } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
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