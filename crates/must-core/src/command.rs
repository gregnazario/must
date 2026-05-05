use crate::error::{Error, Result};
use std::io::{BufRead, BufReader};
use std::process::{Command, ExitStatus, Stdio};

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

pub struct CommandOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_command(cmd: &mut Command, tool: &str, hint: &str) -> Result<CommandOutput> {
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

    let stdout_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let reader = BufReader::new(stdout_pipe);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    println!("{l}");
                    buf.push_str(&l);
                    buf.push('\n');
                }
                Err(_) => break,
            }
        }
        buf
    });

    let stderr_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let reader = BufReader::new(stderr_pipe);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    eprintln!("{l}");
                    buf.push_str(&l);
                    buf.push('\n');
                }
                Err(_) => break,
            }
        }
        buf
    });

    let status = child.wait().map_err(Error::Io)?;
    let stdout = stdout_thread.join().unwrap_or_default();
    let stderr = stderr_thread.join().unwrap_or_default();

    Ok(CommandOutput {
        status,
        stdout,
        stderr,
    })
}

pub fn shell_command(script: &str) -> Command {
    let mut cmd = Command::new(shell_program());
    cmd.arg(shell_arg()).arg(script);
    cmd
}

pub fn shell_program() -> &'static str {
    if cfg!(windows) {
        "cmd"
    } else {
        "sh"
    }
}

pub fn shell_arg() -> &'static str {
    if cfg!(windows) {
        "/C"
    } else {
        "-c"
    }
}

pub fn shell_display(script: &str) -> String {
    if cfg!(windows) {
        format!("cmd /C \"{script}\"")
    } else {
        format!("sh -c '{script}'")
    }
}
