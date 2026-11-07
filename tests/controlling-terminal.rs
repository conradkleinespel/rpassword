//! Default terminal access and explicit readers in a process with no controlling terminal.

#![cfg(all(target_family = "unix", not(target_family = "wasm")))]

use std::io::{self, Cursor};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

#[test]
fn password_input_without_a_controlling_terminal() {
    const CHILD: &str = "RPASSWORD_NO_CONTROLLING_TERMINAL_TEST_CHILD";
    if std::env::var_os(CHILD).is_some() {
        // Keep the original errno available to callers that need to distinguish I/O failures.
        for result in [
            rpassword::read_password(),
            rpassword::prompt_password("Password: "),
        ] {
            assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::ENXIO));
        }

        let config = || {
            rpassword::ConfigBuilder::new()
                .input_reader(Cursor::new("from explicit input\n"))
                .output_discard()
                .build()
        };
        assert_eq!(
            rpassword::read_password_with_config(config()).unwrap(),
            "from explicit input"
        );
        assert_eq!(
            rpassword::prompt_password_with_config("Password: ", config()).unwrap(),
            "from explicit input"
        );
        return;
    }

    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "--exact",
            "password_input_without_a_controlling_terminal",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .stdin(Stdio::null());
    // SAFETY: setsid is async-signal-safe and only changes the child's session. It detaches the
    // controlling terminal without closing any of the parent test runner's file descriptors.
    unsafe {
        child.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    let output = child.output().unwrap();
    assert!(
        output.status.success(),
        "detached test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
