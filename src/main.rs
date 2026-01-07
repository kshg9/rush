use std::io::{self, Write};
use std::{
    env,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();

        let command = line.trim().split('|').filter_map(|c| {
            let chunk = c.trim();
            if !chunk.is_empty() { Some(chunk) } else { None }
        });

        match line.as_str() {
            "exit\n" => break,
            _ => run_command(command),
        }
    }
}

fn run_command<'a>(input: impl Iterator<Item = &'a str>) {
    let mut input = input.peekable();
    let mut prev_output = None;

    while let Some(command) = input.next() {
        let (cmd, args) = match command.split_once(' ') {
            Some((cmd, rest)) => (cmd, Some(rest)),
            None => (command, None),
        };

        let is_piped = input.peek().is_some();

        match cmd {
            "echo" if !is_piped => {
                if let Some(arg_txt) = args {
                    println!("{}", arg_txt);
                }
                prev_output = None;
            }
            "type" if !is_piped => {
                if let Some(arg_txt) = args {
                    let target = arg_txt;
                    match target {
                        "echo" | "exit" | "type" => println!("{} is a shell builtin", target),
                        _ => {
                            if let Some(path) = find_in_path(target) {
                                println!("{} is {}", target, path)
                            } else {
                                println!("{}: not found", target)
                            }
                        }
                    };
                    prev_output = None;
                }
            }
            _ => {
                if let Some(path) = find_in_path(cmd) {
                    let mut child_cmd = Command::new(path);

                    if let Some(arg_txt) = args {
                        child_cmd.args(arg_txt.split_whitespace());
                    }

                    // Input config
                    if let Some(prev_out) = prev_output.take() {
                        child_cmd.stdin(Stdio::from(prev_out));
                    } else {
                        child_cmd.stdin(Stdio::inherit());
                    }

                    // Output config
                    if input.peek().is_some() {
                        child_cmd.stdout(Stdio::piped());
                    } else {
                        child_cmd.stdout(Stdio::inherit());
                    }

                    match child_cmd.spawn() {
                        Ok(mut child) => {
                            if input.peek().is_some() {
                                prev_output = child.stdout.take();
                            } else {
                                let _ = child.wait();
                                prev_output = None;
                            }
                        }
                        Err(e) => println!("{}: failed to execute: {}", cmd, e),
                    }
                } else {
                    println!("{}: command not found", cmd);
                    prev_output = None;
                }
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
