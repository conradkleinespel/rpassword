// Copyright 2014 The Rustastic Password Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate termios;
extern crate libc;

use libc::consts::os::posix88::STDIN_FILENO;
use std::old_io::stdio::{ stdin, StdinReader };
use std::old_io::BufReader;
use std::old_io::IoResult;
use std::slice::bytes::MutableByteVector;

#[cfg(test)]
static TEST_BUFFER: &'static [u8] = b"my-secret\npassword";

#[cfg(test)]
static mut TEST_EOF: bool = false;

#[cfg(test)]
static mut TEST_HAS_SEEN_EOF_BUFFER: bool = false;

#[cfg(test)]
static mut TEST_HAS_SEEN_REGULAR_BUFFER: bool = false;


#[cfg(test)]
fn get_reader() -> BufReader<'static> {
    if unsafe { TEST_EOF } {
        unsafe { TEST_HAS_SEEN_EOF_BUFFER = true; }
        let mut reader = BufReader::new(b"");
        reader.read_to_end().unwrap();
        reader
    } else {
        unsafe { TEST_HAS_SEEN_REGULAR_BUFFER = true; }
        BufReader::new(TEST_BUFFER)
    }
}

#[cfg(not(test))]
fn get_reader() -> StdinReader {
    stdin()
}

pub fn read_password() -> IoResult<String> {
    // Make two copies of the terminal settings. The first one will be modified
    // and the second one will act as a backup for when we want to set the
    // terminal back to its original state.
    let mut term = try!(termios::Termios::from_fd(STDIN_FILENO));
    let term_orig = term;

    // Hide the password. This is what makes this function useful.
    term.c_lflag &= !termios::ECHO;

    // But don't hide the NL character when the user hits ENTER.
    term.c_lflag |= termios::ECHONL;

    // Save the settings for now.
    try!(termios::tcsetattr(STDIN_FILENO, termios::TCSANOW, &term));

    // Read the password.
    let mut password = match get_reader().read_line() {
        Ok(val) => { val },
        Err(err) => {
            // Reset the terminal and quit.
            try!(termios::tcsetattr(STDIN_FILENO, termios::TCSANOW, &term_orig));

            // Return the original IoError.
            return Err(err);
        }
    };

    // Reset the terminal and quit.
    match termios::tcsetattr(STDIN_FILENO, termios::TCSANOW, &term_orig) {
        Ok(_) => {},
        Err(err) => {
            unsafe { password.as_mut_vec() }.set_memory(0);
            return Err(err);
        }
    }

    // Remove the \n from the line.
    password.pop().unwrap();

    Ok(password)
}

#[test]
fn it_works() {
    let term_before = termios::Termios::from_fd(STDIN_FILENO).unwrap();
    assert_eq!(read_password().unwrap().as_slice(), "my-secret");
    let term_after = termios::Termios::from_fd(STDIN_FILENO).unwrap();
    assert_eq!(term_before, term_after);
    unsafe { TEST_EOF = true; }
    assert!(!read_password().is_ok());
    let term_after = termios::Termios::from_fd(STDIN_FILENO).unwrap();
    assert_eq!(term_before, term_after);
    assert!(unsafe { TEST_HAS_SEEN_REGULAR_BUFFER });
    assert!(unsafe { TEST_HAS_SEEN_EOF_BUFFER });
}
