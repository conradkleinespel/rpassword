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

#[cfg(not(windows))]
mod unix {
    extern crate termios;
    extern crate libc;

    use self::libc::consts::os::posix88::STDIN_FILENO;
    use std::io::{ stdin, Stdin, BufReader, BufRead, Read, Error, ErrorKind };
    use std::io::Result as IoResult;
    use std::ptr;
    use std::fs::File;

    /// A trait for operations on mutable `[u8]`s.
    trait MutableByteVector {
        /// Sets all bytes of the receiver to the given value.
        fn set_memory(&mut self, value: u8);
    }

    impl MutableByteVector for Vec<u8> {
        #[inline]
        fn set_memory(&mut self, value: u8) {
            unsafe { ptr::write_bytes(self.as_mut_ptr(), value, self.len()) };
        }
    }

    #[cfg(test)]
    static mut TEST_EOF: bool = false;

    #[cfg(test)]
    static mut TEST_HAS_SEEN_EOF_BUFFER: bool = false;

    #[cfg(test)]
    static mut TEST_HAS_SEEN_REGULAR_BUFFER: bool = false;


    #[cfg(test)]
    fn get_reader<'a>() -> BufReader<File> {
        if unsafe { TEST_EOF } {
            unsafe { TEST_HAS_SEEN_EOF_BUFFER = true; }
            BufReader::new(File::open("/dev/null").unwrap())
        } else {
            unsafe { TEST_HAS_SEEN_REGULAR_BUFFER = true; }
            BufReader::new(File::open("tests/password").unwrap())
        }
    }

    #[cfg(not(test))]
    fn get_reader() -> Stdin {
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
        let mut password = String::new();
        match get_reader().read_line(&mut password) {
            Ok(_) => { },
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
        match password.pop() {
            Some(_) => {},
            None => { return Err(Error::new(ErrorKind::Other, "oh no!")) }
        };

        Ok(password)
    }

    #[test]
    fn it_works() {
        let term_before = termios::Termios::from_fd(STDIN_FILENO).unwrap();
        assert_eq!(read_password().unwrap(), "my-secret");
        let term_after = termios::Termios::from_fd(STDIN_FILENO).unwrap();
        assert_eq!(term_before, term_after);
        unsafe { TEST_EOF = true; }
        assert!(!read_password().is_ok());
        let term_after = termios::Termios::from_fd(STDIN_FILENO).unwrap();
        assert_eq!(term_before, term_after);
        assert!(unsafe { TEST_HAS_SEEN_REGULAR_BUFFER });
        assert!(unsafe { TEST_HAS_SEEN_EOF_BUFFER });
    }
}

#[cfg(not(windows))]
pub use unix::read_password;
