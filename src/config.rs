use crate::defaults::DEFAULT_OUTPUT_PATH;
use crate::defaults::DEFAULT_INPUT_PATH;

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
    pub fn get_input_path(&self) -> Option<&str> {
        match self {
            InputOutput::Input(path) => Some(path.as_str()),
            InputOutput::InputOutput(input_path, _) => Some(input_path.as_str()),
            InputOutput::InputOutputCombined(path) => Some(path.as_str()),
            _ => None,
        }
    }

    pub fn get_output_path(&self) -> Option<&str> {
        match self {
            InputOutput::Output(path) => Some(path.as_str()),
            InputOutput::InputOutput(_, output_path) => Some(output_path.as_str()),
            InputOutput::InputOutputCombined(path) => Some(path.as_str()),
            _ => None,
        }
    }
}

/// Configuration for prompting and reading a password.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    pub(crate) feedback: PasswordFeedback,
    pub(crate) input_path: String,
    pub(crate) output_path: String,
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
            input_path: match self.input_output {
                Some(ref v) => v.get_input_path().unwrap_or(DEFAULT_INPUT_PATH).to_string(),
                _ => DEFAULT_INPUT_PATH.to_string(),
            },
            output_path: match self.input_output {
                Some(ref v) => v.get_output_path().unwrap_or(DEFAULT_OUTPUT_PATH).to_string(),
                _ => DEFAULT_OUTPUT_PATH.to_string(),
            },
        }
    }
}