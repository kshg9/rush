mod utils;

// Basic echo tests
#[test]
fn test_echo() {
    assert_stdout_eq!("echo hello", "hello");
}

#[test]
fn test_echo_multiple_words() {
    assert_stdout_eq!("echo hello world", "hello world");
}

// Pipe tests
#[test]
fn test_pipe_echo_to_grep() {
    assert_pipe_works!("echo hello | grep hello", "hello");
}

#[test]
fn test_pipe_multiple_commands() {
    assert_pipe_works!("echo testing | grep test", "testing");
}

#[test]
fn test_type_external_command() {
    assert_stdout_contains!("type ls", "is /");
}

#[test]
fn test_type_not_found() {
    assert_stdout_contains!("type nonexistent_command", "not found");
}

// Command execution tests
#[test]
fn test_ls_command() {
    assert_command_success!("ls");
}

#[test]
fn test_pwd_command() {
    let output = assert_command_success!("pwd");
    let stdout = utils::get_stdout(&output);
    assert!(!stdout.is_empty(), "pwd should output current directory");
}

// Error handling tests
#[test]
fn test_command_not_found() {
    assert_stdout_contains!("nonexistent_cmd", "command not found");
}

// Complex pipe chains
#[test]
fn test_three_stage_pipe() {
    assert_pipe_works!("echo hello world | grep hel | wc -l", "1");
}
