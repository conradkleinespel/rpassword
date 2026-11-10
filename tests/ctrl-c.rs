//! Ctrl-C must restore the terminal before SIGINT is raised. With the default
//! SIGINT action the process ends inside `raise`, so `Drop` never runs.
//!
//! This is its own test binary because it installs a process-wide SIGINT handler.

#![cfg(all(target_family = "unix", not(target_family = "wasm")))]

use rpassword::{ConfigBuilder, read_password_with_config};
use std::ffi::CStr;
use std::io::ErrorKind;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::{ptr, thread};

static TERMINAL_FD: AtomicI32 = AtomicI32::new(-1);
static LFLAG_AT_SIGINT: AtomicU64 = AtomicU64::new(0);

fn local_flags(fd: libc::c_int) -> libc::tcflag_t {
    let mut term = MaybeUninit::<libc::termios>::uninit();
    assert_eq!(unsafe { libc::tcgetattr(fd, term.as_mut_ptr()) }, 0);
    unsafe { term.assume_init() }.c_lflag
}

extern "C" fn record_local_flags(_: libc::c_int) {
    let mut term = MaybeUninit::<libc::termios>::uninit();
    // tcgetattr is async-signal-safe.
    if unsafe { libc::tcgetattr(TERMINAL_FD.load(Ordering::SeqCst), term.as_mut_ptr()) } == 0 {
        let lflag = unsafe { term.assume_init() }.c_lflag;
        LFLAG_AT_SIGINT.store(u64::from(lflag), Ordering::SeqCst);
    }
}

#[test]
fn ctrl_c_restores_terminal_before_raising_sigint() {
    let (mut master, mut slave) = (-1, -1);
    let opened = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    assert_eq!(opened, 0);
    let mut name = [0 as libc::c_char; 256];
    assert_eq!(
        unsafe { libc::ttyname_r(slave, name.as_mut_ptr(), name.len()) },
        0
    );
    let path = unsafe { CStr::from_ptr(name.as_ptr()) }
        .to_str()
        .unwrap()
        .to_owned();

    TERMINAL_FD.store(slave, Ordering::SeqCst);
    let handler = record_local_flags as extern "C" fn(libc::c_int) as libc::sighandler_t;
    assert_ne!(
        unsafe { libc::signal(libc::SIGINT, handler) },
        libc::SIG_ERR
    );

    let watched = libc::ECHO | libc::ICANON | libc::ISIG;
    let before = local_flags(slave) & watched;
    assert_eq!(before, watched);

    // The terminal turns Ctrl-C into SIGINT until rpassword clears ISIG, so
    // only send the byte once raw mode is on.
    let typist = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        while local_flags(slave) & libc::ISIG != 0 {
            assert!(Instant::now() < deadline, "rpassword never cleared ISIG");
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            unsafe { libc::write(master, b"\x03".as_ptr().cast(), 1) },
            1
        );
    });

    let config = ConfigBuilder::new()
        .input_file_path(path.as_str())
        .output_file_path(path.as_str())
        .build();
    let error = read_password_with_config(config).unwrap_err();
    typist.join().unwrap();

    assert_eq!(error.kind(), ErrorKind::Interrupted);
    assert_eq!(
        LFLAG_AT_SIGINT.load(Ordering::SeqCst) & u64::from(watched),
        u64::from(before),
        "terminal flags were not restored when SIGINT was raised"
    );
    assert_eq!(local_flags(slave) & watched, before);
}
