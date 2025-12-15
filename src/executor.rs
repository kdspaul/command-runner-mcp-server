use std::process::{Command, Output};
use std::time::Duration;
use std::thread;
use std::sync::mpsc;

use crate::request::ExecutionContext;

/// Result of command execution
pub enum ExecutionResult {
    Success(String),
    Error(String),
    Timeout,
}

/// Run a command with the given execution context
pub fn run_command(mut cmd: Command, ctx: &ExecutionContext) -> ExecutionResult {
    // Set working directory if specified
    if let Some(ref dir) = ctx.working_dir {
        cmd.current_dir(dir);
    }

    // Set environment variables if specified
    if let Some(ref env) = ctx.env {
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    // Execute with optional timeout
    match ctx.timeout {
        Some(timeout) => run_with_timeout(cmd, timeout),
        None => run_without_timeout(cmd),
    }
}

fn run_without_timeout(mut cmd: Command) -> ExecutionResult {
    match cmd.output() {
        Ok(output) => output_to_result(output),
        Err(e) => ExecutionResult::Error(format!("Failed to execute command: {}", e)),
    }
}

fn run_with_timeout(mut cmd: Command, timeout: Duration) -> ExecutionResult {
    // Spawn the command
    let child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => return ExecutionResult::Error(format!("Failed to spawn command: {}", e)),
    };

    // Use a channel to communicate between threads
    let (tx, rx) = mpsc::channel();

    // Get the child's pid before moving it into the thread
    let child_id = child.id();

    // Spawn a thread to wait for the child
    let handle = thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });

    // Wait for either completion or timeout
    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => {
            let _ = handle.join();
            output_to_result(output)
        }
        Ok(Err(e)) => {
            let _ = handle.join();
            ExecutionResult::Error(format!("Command failed: {}", e))
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // Kill the child process to avoid resource leaks
            kill_process(child_id);
            // Wait for the thread to finish (it will get an error from the killed process)
            let _ = handle.join();
            ExecutionResult::Timeout
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            ExecutionResult::Error("Command thread disconnected unexpectedly".to_string())
        }
    }
}

/// Kill a process by its ID
fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();
    }
}

fn output_to_result(output: Output) -> ExecutionResult {
    if output.status.success() {
        ExecutionResult::Success(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stderr.is_empty() {
            ExecutionResult::Error(format!("Error: {}", stdout))
        } else {
            ExecutionResult::Error(format!("Error: {}", stderr))
        }
    }
}

impl ExecutionResult {
    /// Convert to a string result for the tool response
    pub fn into_string(self) -> String {
        match self {
            ExecutionResult::Success(s) => s,
            ExecutionResult::Error(s) => s,
            ExecutionResult::Timeout => "Error: Command timed out".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_run_command_simple() {
        let cmd = Command::new("echo");
        let mut cmd = cmd;
        cmd.arg("hello");
        let ctx = ExecutionContext::default();
        let result = run_command(cmd, &ctx);
        match result {
            ExecutionResult::Success(s) => assert!(s.contains("hello")),
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_run_command_with_working_dir() {
        let cmd = Command::new("pwd");
        let ctx = ExecutionContext {
            working_dir: Some("/tmp".to_string()),
            ..Default::default()
        };
        let result = run_command(cmd, &ctx);
        match result {
            ExecutionResult::Success(s) => assert!(s.contains("tmp") || s.contains("private/tmp")),
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_run_command_with_env() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "echo $TEST_VAR"]);
        let mut env = HashMap::new();
        env.insert("TEST_VAR".to_string(), "test_value".to_string());
        let ctx = ExecutionContext {
            env: Some(env),
            ..Default::default()
        };
        let result = run_command(cmd, &ctx);
        match result {
            ExecutionResult::Success(s) => assert!(s.contains("test_value")),
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_run_command_timeout() {
        let mut cmd = Command::new("sleep");
        cmd.arg("10");
        let ctx = ExecutionContext {
            timeout: Some(Duration::from_millis(100)),
            ..Default::default()
        };
        let result = run_command(cmd, &ctx);
        match result {
            ExecutionResult::Timeout => {}
            _ => panic!("Expected timeout"),
        }
    }

    #[test]
    fn test_run_command_error() {
        let cmd = Command::new("ls");
        let mut cmd = cmd;
        cmd.arg("/nonexistent/path/that/does/not/exist");
        let ctx = ExecutionContext::default();
        let result = run_command(cmd, &ctx);
        match result {
            ExecutionResult::Error(s) => assert!(s.contains("Error:")),
            _ => panic!("Expected error"),
        }
    }
}
