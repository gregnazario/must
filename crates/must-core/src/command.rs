use crate::error::{Error, Result};
use crate::output::{print_error, print_output};
use std::io::{BufRead, BufReader};
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

/// Check a Command spawn result, converting NotFound to ToolNotFound.
pub fn run_status(
    result: std::io::Result<ExitStatus>,
    tool: &str,
    hint: &str,
) -> Result<ExitStatus> {
    match result {
        Ok(status) => Ok(status),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::ToolNotFound {
            tool: tool.to_string(),
            hint: hint.to_string(),
        }),
        Err(e) => Err(Error::Io(e)),
    }
}

/// Captured output from a spawned command.
pub struct CommandOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

/// Spawn a command with piped stdout/stderr. Streams output live and captures it.
pub fn run_command(cmd: &mut Command, tool: &str, hint: &str) -> Result<CommandOutput> {
    run_command_with_grace(cmd, tool, hint, Duration::from_secs(10))
}

/// Like [`run_command`], but gives up waiting for output `grace` after the
/// child exits. A grandchild that inherits the pipes (e.g. `my-server &`)
/// would otherwise block the reader threads forever.
pub fn run_command_with_grace(
    cmd: &mut Command,
    tool: &str,
    hint: &str,
    grace: Duration,
) -> Result<CommandOutput> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::ToolNotFound {
                tool: tool.to_string(),
                hint: hint.to_string(),
            });
        }
        Err(e) => return Err(Error::Io(e)),
    };

    let stdout_pipe = child.stdout.take().unwrap();
    let stderr_pipe = child.stderr.take().unwrap();

    enum Pipe {
        Out(Option<String>),
        Err(Option<String>),
    }

    let (tx, rx) = std::sync::mpsc::channel::<Pipe>();
    let stdout_tx = tx.clone();

    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout_pipe);
        let mut buf = Vec::new();
        while let Ok(Some(line)) = read_line_lossy(&mut reader, &mut buf) {
            print_output(&line);
            let _ = stdout_tx.send(Pipe::Out(Some(line)));
        }
        let _ = stdout_tx.send(Pipe::Out(None));
    });

    std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr_pipe);
        let mut buf = Vec::new();
        while let Ok(Some(line)) = read_line_lossy(&mut reader, &mut buf) {
            print_error(&line);
            let _ = tx.send(Pipe::Err(Some(line)));
        }
        let _ = tx.send(Pipe::Err(None));
    });

    let status = child.wait().map_err(Error::Io)?;
    let deadline = std::time::Instant::now() + grace;
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut open_pipes = 2;
    while open_pipes > 0 {
        let timeout = deadline.saturating_duration_since(std::time::Instant::now());
        match rx.recv_timeout(timeout) {
            Ok(Pipe::Out(Some(l))) => {
                stdout.push_str(&l);
                stdout.push('\n');
            }
            Ok(Pipe::Err(Some(l))) => {
                stderr.push_str(&l);
                stderr.push('\n');
            }
            Ok(Pipe::Out(None)) | Ok(Pipe::Err(None)) => open_pipes -= 1,
            Err(_) => break,
        }
    }

    Ok(CommandOutput {
        status,
        stdout,
        stderr,
    })
}

fn read_line_lossy(
    reader: &mut impl BufRead,
    buf: &mut Vec<u8>,
) -> std::io::Result<Option<String>> {
    buf.clear();
    if reader.read_until(b'\n', buf)? == 0 {
        return Ok(None);
    }
    let line = String::from_utf8_lossy(buf);
    let line = line.strip_suffix('\n').unwrap_or(&line);
    let line = line.strip_suffix('\r').unwrap_or(line);
    Ok(Some(line.to_owned()))
}

/// Create a shell Command: `sh -c <script>` on Unix, `cmd /C <script>` on Windows.
pub fn shell_command(script: &str) -> Command {
    let mut cmd = Command::new(shell_program());
    cmd.arg(shell_arg()).arg(script);
    cmd
}

/// Returns the shell binary name for the current platform.
pub fn shell_program() -> &'static str {
    if cfg!(windows) { "cmd" } else { "sh" }
}

/// Returns the shell flag for inline scripts (`-c` on Unix, `/C` on Windows).
pub fn shell_arg() -> &'static str {
    if cfg!(windows) { "/C" } else { "-c" }
}

/// Format a script for human-readable display.
pub fn shell_display(script: &str) -> String {
    if cfg!(windows) {
        format!("cmd /C \"{script}\"")
    } else {
        format!("sh -c '{script}'")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn captures_stdout_and_exit_status() {
        let mut cmd = shell_command("echo hello");
        let out =
            run_command(&mut cmd, "sh", "A shell is required").expect("sh should be available");
        assert!(out.status.success());
        assert!(out.stdout.contains("hello"));
    }

    #[test]
    #[cfg(unix)]
    fn returns_despite_lingering_grandchild() {
        let mut cmd = shell_command("echo started; sleep 30 &");
        let start = std::time::Instant::now();
        let out = run_command_with_grace(
            &mut cmd,
            "sh",
            "A shell is required",
            Duration::from_millis(300),
        )
        .expect("sh should be available");
        assert!(out.status.success());
        assert!(out.stdout.contains("started"));
        assert!(start.elapsed() < Duration::from_secs(10));
    }

    #[test]
    #[cfg(unix)]
    fn keeps_reading_after_invalid_utf8() {
        let mut cmd =
            shell_command("printf 'ok\\377\\n'; head -c 200000 /dev/zero; printf '\\nafter\\n'");
        let out =
            run_command(&mut cmd, "sh", "A shell is required").expect("sh should be available");
        assert!(out.status.success());
        assert!(out.stdout.starts_with("ok\u{FFFD}\n"));
        assert!(out.stdout.ends_with("\nafter\n"));
        assert_eq!(
            out.stdout.len(),
            "ok\u{FFFD}\n".len() + 200000 + "\nafter\n".len()
        );
    }

    #[test]
    #[cfg(unix)]
    fn invalid_utf8_on_stderr_does_not_fail_the_command() {
        let mut cmd = shell_command("printf 'bad\\377byte\\n' >&2; echo done");
        let out =
            run_command(&mut cmd, "sh", "A shell is required").expect("sh should be available");
        assert!(out.status.success());
        assert_eq!(out.stdout, "done\n");
        assert_eq!(out.stderr, "bad\u{FFFD}byte\n");
    }

    #[test]
    fn missing_tool_maps_to_tool_not_found() {
        let mut cmd = Command::new("must-definitely-not-a-real-tool-42");
        match run_command(&mut cmd, "must-definitely-not-a-real-tool-42", "nope") {
            Err(Error::ToolNotFound { .. }) => {}
            Err(e) => panic!("expected ToolNotFound, got {e:?}"),
            Ok(_) => panic!("expected ToolNotFound, got Ok"),
        }
    }
}
