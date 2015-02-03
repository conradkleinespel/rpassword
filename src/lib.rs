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

#[cfg(test)]
static TEST_BUFFER: &'static [u8] = b"my-secret\npassword";

#[cfg(test)]
fn get_reader() -> BufReader<'static> {
    BufReader::new(TEST_BUFFER)
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

    // Read the password and remove the NL character from the end of the line.
    let mut input = get_reader();
    let mut password = try!(input.read_line());
    password.pop().unwrap();

    // Set back the terminal to the original state.
    try!(termios::tcsetattr(STDIN_FILENO, termios::TCSANOW, &term_orig));

    Ok(password)
}

#[test]
fn it_works() {
    let term_before = termios::Termios::from_fd(STDIN_FILENO).unwrap();
    assert_eq!(read_password().unwrap().as_slice(), "my-secret");
    let term_after = termios::Termios::from_fd(STDIN_FILENO).unwrap();
    assert_eq!(term_before, term_after);
}
