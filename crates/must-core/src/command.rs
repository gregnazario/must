use crate::error::{Error, Result};
use std::process::ExitStatus;

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
