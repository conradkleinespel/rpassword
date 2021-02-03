//! This test checks whether or not we can read from a reader when
//! stdin is not a terminal.

use std::io::Cursor;

use rpassword::read_password_from_bufread;

#[cfg(unix)]
#[cfg(unix)]
fn close_stdin() {
    unsafe {
        libc::close(libc::STDIN_FILENO);
    }
}

#[cfg(windows)]
#[cfg(windows)]
fn close_stdin() {
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::STD_INPUT_HANDLE;

    unsafe {
        CloseHandle(GetStdHandle(STD_INPUT_HANDLE));
    }
}

#[cfg(not(any(unix, windows)))]
fn close_stdin() {
    unimplemented!()
}

fn mock_input_crlf() -> Cursor<&'static [u8]> {
    Cursor::new(&b"A mocked response.\r\nAnother mocked response.\r\n"[..])
}

fn mock_input_lf() -> Cursor<&'static [u8]> {
    Cursor::new(&b"A mocked response.\nAnother mocked response.\n"[..])
}

#[test]
fn can_read_from_redirected_input_many_times() {
    close_stdin();

    let mut reader_crlf = mock_input_crlf();

    let response = crate::read_password_from_bufread(&mut reader_crlf).unwrap();
    assert_eq!(response, "A mocked response.");
    let response = crate::read_password_from_bufread(&mut reader_crlf).unwrap();
    assert_eq!(response, "Another mocked response.");

    let mut reader_lf = mock_input_lf();
    let response = crate::read_password_from_bufread(&mut reader_lf).unwrap();
    assert_eq!(response, "A mocked response.");
    let response = crate::read_password_from_bufread(&mut reader_lf).unwrap();
    assert_eq!(response, "Another mocked response.");
}
