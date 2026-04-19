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
use std::fs::OpenOptions;
use std::io::{BufRead, Write};

#[cfg(all(target_family = "unix", not(target_family = "wasm")))]
mod defaults {
    use crate::unix;
    pub use unix::DEFAULT_INPUT_PATH;
    pub use unix::DEFAULT_OUTPUT_PATH;
}
#[cfg(all(target_family = "unix", not(target_family = "wasm")))]
mod unix;
#[cfg(all(target_family = "unix", not(target_family = "wasm")))]
mod feedback;
#[cfg(all(target_family = "unix", not(target_family = "wasm")))]
pub use unix::read_password;
#[cfg(all(target_family = "unix", not(target_family = "wasm")))]
pub use unix::read_password_with_config;

#[cfg(target_family = "windows")]
mod defaults {
    use crate::windows;
    pub use windows::DEFAULT_INPUT_PATH;
    pub use windows::DEFAULT_OUTPUT_PATH;
}
#[cfg(target_family = "windows")]
mod windows;
#[cfg(target_family = "windows")]
mod feedback;
#[cfg(target_family = "windows")]
pub use windows::read_password;
#[cfg(target_family = "windows")]
pub use windows::read_password_with_config;

#[cfg(target_family = "wasm")]
mod defaults {
    use crate::wasm;
    pub use wasm::DEFAULT_INPUT_PATH;
    pub use wasm::DEFAULT_OUTPUT_PATH;
}
#[cfg(target_family = "wasm")]
mod wasm;
#[cfg(target_family = "wasm")]
pub use wasm::read_password;
#[cfg(target_family = "wasm")]
pub use wasm::read_password_with_config;

mod config;
pub use config::{InputOutput, PasswordFeedback, Config, ConfigBuilder};

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
    let mut file = OpenOptions::new()
        .write(true)
        .open(config.output_path.as_str())?;
    file.write_all(prompt.to_string().as_bytes())?;
    file.flush()?;
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

}
