use std::io::Write;
use std::process::{Command, Output};

/// Helper to run a command through the rush shell
pub fn run_shell_command(input: &str) -> Output {
    let mut child = Command::new("./target/debug/rush")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn rush shell");

    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "{}", input).expect("Failed to write to stdin");
        writeln!(stdin, "exit").expect("Failed to write exit command");
    }

    child.wait_with_output().expect("Failed to wait on child")
}

/// Helper to get stdout as a string
pub fn get_stdout(output: &Output) -> String {
    let full_output = String::from_utf8_lossy(&output.stdout);
    // Filter out lines that start with "$ " (shell prompts) and collect the rest
    full_output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            // Skip empty lines and standalone prompts
            if trimmed.is_empty() || trimmed == "$" {
                None
            }
            // Remove the prompt prefix if present
            else if let Some(content) = line.strip_prefix("$ ") {
                Some(content)
            }
            // Include lines that don't have prompts
            else if !line.starts_with('$') {
                Some(line)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Assert that a command succeeds (exit code 0)
#[macro_export]
macro_rules! assert_command_success {
    ($cmd:expr) => {{
        let output = utils::run_shell_command($cmd);
        assert!(
            output.status.success(),
            "Command '{}' failed with status: {}\nstdout: {}\nstderr: {}",
            $cmd,
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }};
}

/// Assert that a command's stdout contains a specific string
#[macro_export]
macro_rules! assert_stdout_contains {
    ($cmd:expr, $expected:expr) => {{
        let output = utils::run_shell_command($cmd);
        let stdout = utils::get_stdout(&output);
        assert!(
            stdout.contains($expected),
            "Expected stdout to contain '{}' but got: '{}'",
            $expected,
            stdout
        );
        output
    }};
}

/// Assert that a command's stdout with eq
#[macro_export]
macro_rules! assert_stdout_eq {
    ($cmd:expr, $expected:expr) => {{
        let output = utils::run_shell_command($cmd);
        let stdout = utils::get_stdout(&output);
        assert!(
            stdout.eq($expected),
            "Expected stdout to contain '{}' but got: '{}'",
            $expected,
            stdout
        );
        output
    }};
}

/// Test pipe operations
#[macro_export]
macro_rules! assert_pipe_works {
    ($cmd:expr, $expected:expr) => {{
        let output = utils::run_shell_command($cmd);
        let stdout = utils::get_stdout(&output);
        assert!(
            stdout.eq($expected),
            "Pipe command '{}' failed. Expected output to contain '{}', got: '{}'",
            $cmd,
            $expected,
            stdout
        );
        output
    }};
}

/// Test that a command produces non-empty output
#[macro_export]
macro_rules! assert_has_output {
    ($cmd:expr) => {{
        let output = utils::run_shell_command($cmd);
        let stdout = utils::get_stdout(&output);
        assert!(
            !stdout.is_empty(),
            "Expected command '{}' to produce output, but got none",
            $cmd
        );
        output
    }};
}

/// Test that a command produces no output
#[macro_export]
macro_rules! assert_no_output {
    ($cmd:expr) => {{
        let output = utils::run_shell_command($cmd);
        let stdout = utils::get_stdout(&output);
        assert!(
            stdout.is_empty(),
            "Expected command '{}' to produce no output, but got: '{}'",
            $cmd,
            stdout
        );
        output
    }};
}
