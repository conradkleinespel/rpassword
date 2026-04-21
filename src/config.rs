use crate::DEFAULT_INPUT_PATH;
use crate::DEFAULT_OUTPUT_PATH;
use std::io::Cursor;

/// Controls visual feedback when the user types a password.
///
/// Note: On Wasm, only `PasswordFeedback::Hide` is supported.
///
/// # Examples
///
/// ## Masking the password with asterisks
/// ```
/// use rpassword::{ConfigBuilder, PasswordFeedback};
///
/// let config = ConfigBuilder::new()
///     .password_feedback(PasswordFeedback::Mask('*'))
///     .build();
/// ```
///
/// ## Showing first 3 characters in plaintext, then asterisks
/// ```
/// use rpassword::{ConfigBuilder, PasswordFeedback};
///
/// let config = ConfigBuilder::new()
///     .password_feedback(PasswordFeedback::PartialMask('*', 3))
///     .build();
/// ```
///
/// ## Hiding the password entirely (default behavior)
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

/// Specifies the target for input or output operations.
///
/// This enum defines where input is read from or where output is written to.
/// It supports file paths, in-memory cursors, or no input/output at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InputOutputTarget {
    FilePath(String),
    Cursor(Cursor<Vec<u8>>),
    Void,
}

/// Configuration for prompting and reading a password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub(crate) password_feedback: PasswordFeedback,
    pub(crate) input: InputOutputTarget,
    pub(crate) output: InputOutputTarget,
}

/// A builder for creating a [`Config`].
///
/// This struct provides a convenient way to configure the behavior of password reading,
/// such as setting visual feedback, specifying an input path, discarding output, etc.
///
/// # Examples
///
/// ## Customising how the password is hidden
/// ```
/// use rpassword::{ConfigBuilder, PasswordFeedback};
///
/// let config = ConfigBuilder::new()
///     .password_feedback(PasswordFeedback::Mask('*'))
///     .build();
/// ```
///
/// ## Setting custom input file paths
/// ```
/// use rpassword::{ConfigBuilder};
///
/// let config = ConfigBuilder::new()
///     .input_file_path("path/to/file/containing/password")
///     .build();
/// ```
///
/// ## Reading from in-memory data
/// ```
/// use rpassword::{ConfigBuilder};
///
/// let config = ConfigBuilder::new()
///     .input_data("my-password\n")
///     .build();
/// ```
///
/// ## Discarding output
/// ```
/// use rpassword::{ConfigBuilder};
///
/// let config = ConfigBuilder::new()
///     .output_discard()
///     .build();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigBuilder {
    feedback: PasswordFeedback,
    input: InputOutputTarget,
    output: InputOutputTarget,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        ConfigBuilder {
            feedback: PasswordFeedback::default(),
            input: InputOutputTarget::FilePath(DEFAULT_INPUT_PATH.to_string()),
            output: InputOutputTarget::FilePath(DEFAULT_OUTPUT_PATH.to_string()),
        }
    }
}

impl ConfigBuilder {
    pub fn new() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Sets the visual feedback for the password.
    pub fn password_feedback(self, feedback: PasswordFeedback) -> ConfigBuilder {
        ConfigBuilder { feedback, ..self }
    }

    /// Reads the passwords from the data.
    pub fn input_data(self, data: impl Into<Vec<u8>>) -> ConfigBuilder {
        ConfigBuilder {
            input: InputOutputTarget::Cursor(Cursor::new(data.into())),
            ..self
        }
    }

    /// Reads the password from the file at the given path.
    pub fn input_file_path(self, file_path: impl Into<String>) -> ConfigBuilder {
        ConfigBuilder {
            input: InputOutputTarget::FilePath(file_path.into()),
            ..self
        }
    }

    /// Sends the output to the file at the given path.
    pub fn output_file_path(self, file_path: impl Into<String>) -> ConfigBuilder {
        ConfigBuilder {
            output: InputOutputTarget::FilePath(file_path.into()),
            ..self
        }
    }

    /// Discards any output.
    pub fn output_discard(self) -> ConfigBuilder {
        ConfigBuilder {
            output: InputOutputTarget::Void,
            ..self
        }
    }

    /// Builds the final [`Config`].
    pub fn build(self) -> Config {
        Config {
            password_feedback: self.feedback,
            input: self.input,
            output: self.output,
        }
    }
}
