use crate::DEFAULT_INPUT_PATH;
use crate::DEFAULT_OUTPUT_PATH;
use std::io::Cursor;

/// Controls visual feedback when the user types a password.
///
/// Note: On Wasm, only `PasswordFeedback::Hide` is supported.
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

/// Specifies the target for input or output operations.
///
/// This enum defines where input is read from or where output is written to.
/// It supports file paths, in-memory cursors, or no input/output at all.
///
/// # Variants
///
/// - `FilePath(String)`: Reads from or writes to a file at the specified path.
/// - `Cursor(Cursor<u8>)`: Reads from or writes to an in-memory cursor, useful for testing.
/// - `Void`: No Input or no Output, useful when input/output is unnecessary.
///
/// # Examples
///
/// ## Using a File Path
/// ```
/// use rpassword::InputOutputTarget;
///
/// let target = InputOutputTarget::FilePath("/dev/tty".to_string());
/// ```
///
/// ## Using a Cursor
/// ```
/// use rpassword::InputOutputTarget;
/// use std::io::Cursor;
///
/// let cursor = Cursor::new(b"test data".to_vec());
/// let target = InputOutputTarget::Cursor(cursor);
/// ```
///
/// ## No Input/Output
/// ```
/// use rpassword::InputOutputTarget;
///
/// let target = InputOutputTarget::Void;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputOutputTarget {
    FilePath(String),
    Cursor(Cursor<Vec<u8>>),
    Void,
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
/// use rpassword::{ConfigBuilder, InputOutput, InputOutputTarget};
///
/// let config = ConfigBuilder::new()
///     .input_output(InputOutput::new().input(InputOutputTarget::FilePath("/dev/tty42".to_string())))
///     .build();
/// ```
///
/// ## Setting a Custom Output Path
/// ```
/// use rpassword::{ConfigBuilder, InputOutput, InputOutputTarget};
///
/// let config = ConfigBuilder::new()
///     .input_output(InputOutput::new().output(InputOutputTarget::FilePath("/dev/tty".to_string())))
///     .build();
/// ```
///
/// ## Setting Both Custom Input and Output Paths
/// ```
/// use rpassword::{ConfigBuilder, InputOutput, InputOutputTarget};
///
/// let config = ConfigBuilder::new()
///     .input_output(
///         InputOutput::new()
///             .input(InputOutputTarget::FilePath("/dev/tty42".to_string()))
///             .output(InputOutputTarget::FilePath("/dev/tty84".to_string()))
///     )
///     .build();
/// ```
///
/// ## Setting Input to a Cursor and discarding Output
/// ```
/// use std::io::Cursor;
/// use rpassword::{ConfigBuilder, InputOutput, InputOutputTarget};
///
/// let input = Cursor::new(b"my-password\n".to_vec());
///
/// let config = ConfigBuilder::new()
///     .input_output(
///         InputOutput::new()
///             .input(InputOutputTarget::Cursor(input))
///             .output(InputOutputTarget::Void)
///     )
///     .build();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputOutput {
    pub(crate) input: InputOutputTarget,
    pub(crate) output: InputOutputTarget,
}

impl Default for InputOutput {
    fn default() -> Self {
        InputOutput::new()
    }
}

impl InputOutput {
    pub fn new() -> InputOutput {
        InputOutput {
            input: InputOutputTarget::FilePath(DEFAULT_INPUT_PATH.to_string()),
            output: InputOutputTarget::FilePath(DEFAULT_OUTPUT_PATH.to_string()),
        }
    }

    pub fn input(self, input: InputOutputTarget) -> InputOutput {
        InputOutput {
            input,
            output: self.output,
        }
    }

    pub fn output(self, output: InputOutputTarget) -> InputOutput {
        InputOutput {
            input: self.input,
            output,
        }
    }
}

/// Configuration for prompting and reading a password.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    pub(crate) password_feedback: PasswordFeedback,
    pub(crate) input_output: InputOutput,
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
/// use rpassword::{ConfigBuilder, InputOutput, InputOutputTarget};
///
/// let config = ConfigBuilder::new()
///     .input_output(InputOutput::new().input(InputOutputTarget::FilePath("/dev/tty42".to_string())))
///     .build();
/// ```
///
/// ## Combining Feedback and Input/Output Paths
/// ```
/// use rpassword::{ConfigBuilder, PasswordFeedback, InputOutput, InputOutputTarget};
///
/// let config = ConfigBuilder::new()
///     .password_feedback(PasswordFeedback::PartialMask('*', 3))
///     .input_output(InputOutput::new().input(InputOutputTarget::FilePath("/dev/tty42".to_string())))
///     .build();
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigBuilder {
    feedback: PasswordFeedback,
    input_output: InputOutput,
}

impl ConfigBuilder {
    pub fn new() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Sets the visual feedback for the password.
    pub fn password_feedback(self, feedback: PasswordFeedback) -> ConfigBuilder {
        ConfigBuilder { feedback, ..self }
    }

    /// Sets the path to the input and output files (defaults to the console).
    ///
    /// This can also be used to pass a temporary file for testing.
    pub fn input_output(self, input_output: InputOutput) -> ConfigBuilder {
        ConfigBuilder {
            input_output,
            ..self
        }
    }

    /// Builds the final [`Config`].
    pub fn build(self) -> Config {
        Config {
            password_feedback: self.feedback,
            input_output: self.input_output,
        }
    }
}
