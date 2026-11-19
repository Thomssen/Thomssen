//! Running helper executables safely: fixed argument lists (never a shell
//! string), no console window, and a hard timeout.

use crate::error::{Error, Result};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

/// Runs `cmd`, capturing stdout. Fails if it exceeds `timeout` (the child is
/// killed) or exits non-zero.
pub fn run_capture(mut cmd: Command, timeout: Duration) -> Result<String> {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| Error::Command(format!("could not start helper: {e}")))?;
    let mut out = child.stdout.take();
    let mut err = child.stderr.take();
    let h_out = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(o) = out.as_mut() {
            let _ = o.read_to_end(&mut s);
        }
        s
    });
    let h_err = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(o) = err.as_mut() {
            let _ = o.read_to_end(&mut s);
        }
        s
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait()? {
            Some(s) => break s,
            None if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::Command("helper timed out".into()));
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    let stdout = String::from_utf8_lossy(&h_out.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&h_err.join().unwrap_or_default()).into_owned();
    if !status.success() {
        let msg = stderr.trim();
        if msg.to_ascii_lowercase().contains("access is denied") {
            return Err(Error::PermissionDenied("Windows denied access. This change needs administrator rights.".into()));
        }
        return Err(Error::Command(format!("helper exited with {status}{}", if msg.is_empty() { String::new() } else { format!(": {}", msg.lines().next().unwrap_or("")) })));
    }
    Ok(stdout)
}

/// Runs a **fixed** PowerShell script (never built from user input) and
/// returns stdout. Used only for read-only hardware queries.
pub fn powershell(script: &str, timeout: Duration) -> Result<String> {
    let mut c = command("powershell.exe");
    c.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    run_capture(c, timeout)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn captures_and_times_out() {
        let mut c = command("echo");
        c.arg("hello");
        assert_eq!(run_capture(c, Duration::from_secs(5)).unwrap().trim(), "hello");
        let mut s = command("sleep");
        s.arg("5");
        assert!(run_capture(s, Duration::from_millis(100)).is_err());
        assert!(run_capture(command("false"), Duration::from_secs(5)).is_err());
    }
}
