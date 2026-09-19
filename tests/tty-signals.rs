#![cfg(target_os = "linux")]

use std::fs::File;
use std::io::{self, Write};
use std::mem::MaybeUninit;
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct ChildGuard {
    child: std::process::Child,
    reaped: bool,
}

impl ChildGuard {
    fn new(child: std::process::Child) -> Self {
        Self {
            child,
            reaped: false,
        }
    }

    fn wait_bounded(&mut self) -> io::Result<std::process::ExitStatus> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Some(status) = self.child.try_wait()? {
                self.reaped = true;
                return Ok(status);
            }
            thread::sleep(Duration::from_millis(10));
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "password prompt did not terminate after signal",
        ))
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct TerminalState {
    input_flags: libc::tcflag_t,
    output_flags: libc::tcflag_t,
    control_flags: libc::tcflag_t,
    local_flags: libc::tcflag_t,
    control_chars: [libc::cc_t; libc::NCCS],
    input_speed: libc::speed_t,
    output_speed: libc::speed_t,
}

fn terminal_attributes(fd: RawFd) -> io::Result<libc::termios> {
    let mut attributes = MaybeUninit::uninit();
    if unsafe { libc::tcgetattr(fd, attributes.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { attributes.assume_init() })
}

fn terminal_state(fd: RawFd) -> io::Result<TerminalState> {
    let attributes = terminal_attributes(fd)?;
    Ok(TerminalState {
        input_flags: attributes.c_iflag,
        output_flags: attributes.c_oflag,
        control_flags: attributes.c_cflag,
        local_flags: attributes.c_lflag,
        control_chars: attributes.c_cc,
        input_speed: unsafe { libc::cfgetispeed(&attributes) },
        output_speed: unsafe { libc::cfgetospeed(&attributes) },
    })
}

fn set_non_default_profile(fd: RawFd) -> io::Result<TerminalState> {
    let mut attributes = terminal_attributes(fd)?;
    attributes.c_lflag ^= libc::ECHOCTL;
    attributes.c_cc[libc::VEOF] = 7;
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &attributes) } == -1 {
        return Err(io::Error::last_os_error());
    }
    terminal_state(fd)
}

fn open_pty() -> io::Result<(File, File)> {
    let mut master = -1;
    let mut slave = -1;
    if unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
        )
    } == -1
    {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) })
}

fn spawn_prompt(slave: &File) -> io::Result<std::process::Child> {
    let slave_fd = std::os::fd::AsRawFd::as_raw_fd(slave);
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--exact")
        .arg("child_password_prompt")
        .arg("--ignored")
        .stdin(Stdio::from(slave.try_clone()?))
        .stdout(Stdio::from(slave.try_clone()?))
        .stderr(Stdio::from(slave.try_clone()?));

    unsafe {
        command.pre_exec(move || {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            if libc::ioctl(slave_fd, libc::TIOCSCTTY, 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    command.spawn()
}

fn wait_until_raw(fd: RawFd) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let attributes = terminal_attributes(fd).unwrap();
        if attributes.c_lflag & (libc::ECHO | libc::ICANON | libc::ISIG) == 0 {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("child did not enter raw password input mode");
}

fn assert_ctrl_c_restores_terminal() {
    let (mut master, slave) = open_pty().unwrap();
    let expected = set_non_default_profile(std::os::fd::AsRawFd::as_raw_fd(&slave)).unwrap();
    let mut child = ChildGuard::new(spawn_prompt(&slave).unwrap());
    wait_until_raw(std::os::fd::AsRawFd::as_raw_fd(&slave));

    master.write_all(&[3]).unwrap();
    master.flush().unwrap();

    let status = child.wait_bounded().unwrap();
    assert_eq!(status.signal(), Some(libc::SIGINT));
    assert_eq!(
        terminal_state(std::os::fd::AsRawFd::as_raw_fd(&slave)).unwrap(),
        expected
    );
}

#[test]
#[ignore]
fn child_password_prompt() {
    let _ = rpassword::read_password();
}

#[test]
fn typed_ctrl_c_restores_terminal() {
    assert_ctrl_c_restores_terminal();
}
