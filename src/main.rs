#[allow(unused_imports)]
use std::io::{self, Write};
use std::{env, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();

        match line.as_str() {
            "exit\n" => break,
            _ => run_command(&line),
        }
    }
}

fn run_command(input: &str) {
    let input = input.trim();
    let (cmd, args) = match input.split_once(' ') {
        Some((cmd, rest)) => (cmd, Some(rest)),
        None => (input, None),
    };

    match cmd {
        "echo" => {
            if let Some(arg_txt) = args {
                println!("{}", arg_txt);
            }
        }
        "type" => {
            if let Some(arg_txt) = args {
                let target = arg_txt.trim();
                match target {
                    "echo" | "exit" | "type" => println!("{} is a shell builtin", target),
                    _ => {
                        if let Some(path) = find_in_path(target) {
                            println!("{} is {}", target, path)
                        } else {
                            println!("{}: not found", target)
                        }
                    }
                }
            }
        }
        _ => {
            if let Some(path) = find_in_path(cmd) {
                let mut cmd = Command::new(cmd);

                if let Some(arg_txt) = args {
                    cmd.args(arg_txt.split_whitespace());
                }
                let _ = cmd.status();
            } else {
                println!("{}: command not found", cmd);
            }
        }
    }
}

fn find_in_path(command: &str) -> Option<String> {
    let path_var = env::var("PATH").unwrap_or_default();

    for dir in env::split_paths(&path_var) {
        let full_path = dir.join(command);
        // println!("{:?}", full_path);

        if full_path.is_file() && is_executable(&full_path) {
            return Some(full_path.to_string_lossy().to_string());
        }
    }
    None
}

fn is_executable(path: &PathBuf) -> bool {
    if let Ok(metadata) = path.metadata() {
        return metadata.permissions().mode() & 0o111 != 0;
    }
    false
}
