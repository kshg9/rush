use std::env;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{ChildStdout, Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub cmd: String,
    pub args: Vec<String>,
}
pub struct Shell {
    path_entries: Vec<PathBuf>,
}

impl CommandSpec {
    /// `args: impl IntoIterator<Item = S>` signature to accept any type that can be converted
    /// into any iterator yielding items of type `s`.
    /// Essentially simplified version of:
    /// ```ignore
    /// pub fn new<S: Into<String>, I>(cmd: S, args: I) -> Self
    ///  where
    ///      S: Clone,
    ///      I: IntoIterator<Item = S>,
    /// ```
    /// It reduces the usage of turbofish we'd need.
    pub fn new<S: Into<String>>(cmd: S, args: impl IntoIterator<Item = S>) -> Self
    where
        S: Clone,
    {
        Self {
            // converts S to String (whether &str or String::from etc)
            cmd: cmd.into(),
            args: args.into_iter().map(Into::into).collect(),
            // for each item in iterator, call into() on it
            // sidenote:
            // `.map(|s| s.into())` is same as `.map(Into::into)`
        }
    }
}

pub fn parse_pipeline(line: &str) -> Vec<CommandSpec> {
    line.split('|')
        .filter_map(|chunk| {
            let trimmed = chunk.trim();
            if trimmed.is_empty() {
                return None;
            }

            let mut tokens = tokenize_with_quotes(trimmed).into_iter();
            let cmd = tokens.next()?;
            let args = tokens.collect();

            Some(CommandSpec { cmd, args })
        })
        .collect()
}

impl Default for Shell {
    fn default() -> Self {
        let path_var = env::var_os("PATH").unwrap_or_default();
        let entries = env::split_paths(&path_var).collect();
        Self {
            path_entries: entries,
        }
    }
}

impl Shell {
    pub fn with_paths(paths: &[PathBuf]) -> Self {
        Self {
            path_entries: paths.to_vec(),
        }
    }

    /// reminder [`io::Result`] basically wraps the error filed so we dont have to think about it.
    pub fn run_repl(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        loop {
            print!("$ ");
            io::stdout().flush()?;

            let mut line = String::new();
            stdin.read_line(&mut line)?;
            if should_exit(&line) {
                break;
            }

            let pipeline = parse_pipeline(&line);
            if pipeline.is_empty() {
                continue;
            }

            self.run_pipeline(&pipeline)?;
        }
        Ok(())
    }

    /// The pipeline starts here
    /// `&[CommandSpec]` is used here as it represents slices whereas `&Vec<CommandSpec>` is too specific.
    /// It still works.
    pub fn run_pipeline(&self, pipeline: &[CommandSpec]) -> io::Result<()> {
        let mut iter = pipeline.iter().peekable();
        let mut prev_output: Option<ChildStdout> = None;

        while let Some(spec) = iter.next() {
            let is_piped = iter.peek().is_some();
            if !is_piped {
                if let Some(output) = self.eval_builtin(spec) {
                    if !output.is_empty() {
                        println!("{}", output);
                    }
                    prev_output = None;
                    continue;
                }
            }

            prev_output = self.run_external(spec, prev_output, is_piped)?;
        }

        Ok(())
    }

    fn run_external(
        &self,
        spec: &CommandSpec,
        prev_output: Option<ChildStdout>,
        keep_stdout: bool,
    ) -> io::Result<Option<ChildStdout>> {
        let path = match self.find_in_path(&spec.cmd) {
            Some(p) => p,
            None => {
                println!("{}: command not found", spec.cmd);
                return Ok(None);
            }
        };

        let mut cmd = Command::new(path);
        cmd.args(&spec.args);

        if let Some(stdout) = prev_output {
            cmd.stdin(Stdio::from(stdout));
        }

        if keep_stdout {
            cmd.stdout(Stdio::piped());
        }

        let mut child = cmd.spawn()?;

        if keep_stdout {
            Ok(child.stdout.take())
        } else {
            let _ = child.wait();
            Ok(None)
        }
    }

    fn eval_builtin(&self, spec: &CommandSpec) -> Option<String> {
        match spec.cmd.as_str() {
            "echo" => {
                if spec.args.is_empty() {
                    None
                } else {
                    Some(spec.args.join(" "))
                }
            }
            "type" => {
                let target = spec.args.first()?.as_str();
                if is_builtin_name(target) {
                    Some(format!("{} is a shell builtin", target))
                } else if let Some(path) = self.find_in_path(target) {
                    Some(format!("{} is {}", target, path.display()))
                } else {
                    Some(format!("{}: not found", target))
                }
            }
            _ => None,
        }
    }

    fn find_in_path(&self, command: &str) -> Option<PathBuf> {
        for dir in &self.path_entries {
            let full_path = dir.join(command);
            if full_path.is_file() && is_executable(&full_path) {
                return Some(full_path);
            }
        }
        None
    }
}

fn should_exit(line: &str) -> bool {
    line.trim_end() == "exit"
}

/// Performs bitwise '&' operation with octal value 111
/// In unix permissions for executing programs:
/// (r)ead=4, (w)rite=2, (e)xecute=1
/// 100 -> owner, 010 -> group, 001 -> others (binary representation)
/// so 0o222 to check for writes, 0o755 for checking read+write+execute in owner, read+execute in group and others.
/// checks for executable in those three category.
fn is_executable(path: &PathBuf) -> bool {
    if let Ok(metadata) = path.metadata() {
        return metadata.permissions().mode() & 0o111 != 0;
    }
    false
}

fn is_builtin_name(name: &str) -> bool {
    matches!(name, "echo" | "exit" | "type")
}

fn tokenize_with_quotes(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current: Option<String> = None;
    let mut in_quotes = false;

    for ch in input.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                // Ensure we have a Some(String) to track this token, even if empty
                current.get_or_insert_with(String::new);
            }
            c if c.is_whitespace() && !in_quotes => {
                if let Some(token) = current.take() {
                    tokens.push(token);
                }
            }
            c => {
                current.get_or_insert_with(String::new).push(c);
            }
        }
    }

    if let Some(token) = current {
        tokens.push(token);
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::tempdir;

    #[test]
    fn parse_pipeline_splits_commands() {
        let cmds = parse_pipeline(" echo hi | grep h | wc -l ");
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].cmd, "echo");
        assert_eq!(cmds[0].args, vec!["hi"]);
        assert_eq!(cmds[1].cmd, "grep");
        assert_eq!(cmds[1].args, vec!["h"]);
        assert_eq!(cmds[2].cmd, "wc");
        assert_eq!(cmds[2].args, vec!["-l"]);
    }

    #[test]
    fn parse_pipeline_skips_empty_chunks() {
        let cmds = parse_pipeline("|| echo hi ||");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].cmd, "echo");
        assert_eq!(cmds[0].args, vec!["hi"]);
    }

    #[test]
    fn builtin_echo_concatenates_args() {
        let shell = Shell::with_paths(&Vec::new());
        let spec = CommandSpec {
            cmd: "echo".into(),
            args: vec!["hello".into(), "world".into()],
        };
        assert_eq!(shell.eval_builtin(&spec), Some("hello world".into()));
    }

    #[test]
    fn builtin_type_identifies_builtin() {
        let shell = Shell::with_paths(&Vec::new());
        let spec = CommandSpec {
            cmd: "type".into(),
            args: vec!["echo".into()],
        };
        assert_eq!(
            shell.eval_builtin(&spec),
            Some("echo is a shell builtin".into())
        );
    }

    #[test]
    fn find_in_path_respects_custom_paths() {
        let dir = tempdir().unwrap();
        let exe_path = dir.path().join("dummy");
        File::create(&exe_path).unwrap();
        let mut perms = exe_path.metadata().unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&exe_path, perms).unwrap();

        let shell = Shell::with_paths(&vec![dir.path().to_path_buf()]);
        let resolved = shell.find_in_path("dummy");
        assert_eq!(resolved.as_deref(), Some(exe_path.as_path()));
    }

    #[test]
    fn is_executable_checks_permission_bits() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file");
        File::create(&file_path).unwrap();
        let mut perms = file_path.metadata().unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&file_path, perms).unwrap();
        assert!(!is_executable(&file_path));
        let mut perms = file_path.metadata().unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&file_path, perms).unwrap();
        assert!(is_executable(&file_path));
    }

    #[test]
    fn should_exit_matches_trimmed_exit() {
        assert!(should_exit("exit\n"));
        assert!(should_exit("exit"));
        assert!(!should_exit("exit now"));
    }

    #[test]
    fn tokenize_handles_quoted_strings() {
        let result = tokenize_with_quotes(r#"grep "hello world" file.txt"#);
        assert_eq!(result, vec!["grep", "hello world", "file.txt"]);
    }

    #[test]
    fn tokenize_handles_empty_quotes() {
        let result = tokenize_with_quotes(r#"echo """#);
        assert_eq!(result, vec!["echo", ""]);
    }

    #[test]
    fn tokenize_handles_multiple_spaces() {
        let result = tokenize_with_quotes("echo   hello    world");
        assert_eq!(result, vec!["echo", "hello", "world"]);
    }

    #[test]
    fn parse_pipeline_with_quotes() {
        let cmds = parse_pipeline(r#"ls -la | grep ".rs""#);
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].cmd, "ls");
        assert_eq!(cmds[0].args, vec!["-la"]);
        assert_eq!(cmds[1].cmd, "grep");
        assert_eq!(cmds[1].args, vec![".rs"]);
    }
}
