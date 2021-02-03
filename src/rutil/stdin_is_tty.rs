/// Checks if the program is run via a TTY
#[cfg(unix)]
mod unix {

    use libc::{isatty, STDIN_FILENO};

    pub fn stdin_is_tty() -> bool {
        unsafe { isatty(STDIN_FILENO) == 1 }
    }
}

/// Checks if the program is run via a TTY
#[cfg(windows)]
mod windows {

    use winapi::um::fileapi::GetFileType;
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::{FILE_TYPE_CHAR, STD_INPUT_HANDLE};

    pub fn stdin_is_tty() -> bool {
        let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        if handle == INVALID_HANDLE_VALUE {
            panic!("Invalid STDIN handle");
        }

        unsafe { GetFileType(handle) == FILE_TYPE_CHAR }
    }
}

/// Returns `true` if the program is run via a TTY, `false` otherwise
#[cfg(unix)]
pub fn stdin_is_tty() -> bool {
    unix::stdin_is_tty()
}

/// Returns `true` if the program is run via a TTY, `false` otherwise
#[cfg(windows)]
pub fn stdin_is_tty() -> bool {
    windows::stdin_is_tty()
}
