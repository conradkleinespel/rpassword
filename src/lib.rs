//! This library makes it easy to read passwords in a console application on all platforms, Unix and
//! Windows alike.
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

#[allow(unused)]
mod rpassword;
#[allow(unused)]
mod rutil;

pub use crate::rpassword::*;
