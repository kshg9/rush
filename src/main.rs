use rush::Shell;

fn main() {
    let mut shell = Shell::default();
    if let Err(err) = shell.run_repl() {
        eprintln!("rush: {}", err);
    }
}
