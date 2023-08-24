//use regex::bytes::Regex;
use regex::Regex;
use rexpect::error::Error as RexpectError;
use rexpect::session::{spawn_command, PtySession};
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

/// Strip a string until the 'alternate screen exit' escape code, and return the slice containing
/// the remaining string.
fn strip_until_alternate_screen_exit(text: &str) -> &str {
    // \u{1b}[?1049l - exit alternate screen
    let ptn = Regex::new(r"\x1b\[\?1049l").unwrap();
    if let Some(m) = ptn.find(text) {
        &text[m.end()..]
    } else {
        text
    }
}

/// Initialize the app and wait until it has entered the alternate screen. Returns a handle to the
/// rexpect PtySession, which is ready for input. Panics if initializing the app fails.
fn run_app() -> PtySession {
    let mut cmd = get_cmd();
    // explicitly pass empty history file so we don't get first run prompt
    // NOTE: cannot directly chain this with get_cmd(), otherwise we get a mutable ref which we
    // can't move into run_app_with_cmd.
    cmd.args(["--history-file", ""]);
    run_app_with_cmd(cmd)
}

/// Initialize app with the given Command object and wait until it has entered the alternate screen.
/// Returns a handle to the rexpect PtySession, which is ready for input. Panics if initializing
/// the app fails.
fn run_app_with_cmd(cmd: Command) -> PtySession {
    let mut proc = spawn_command(cmd, Some(1_000)).expect("error spawning process");

    // \u{1b}[?1049h - enter alternate screen
    proc.exp_string("\x1b[?1049h").unwrap();
    proc
}

fn get_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tere"))
}

#[test]
fn output_on_exit_without_cd() -> Result<(), RexpectError> {
    let mut proc = run_app();

    proc.send_control('c')?;
    proc.writer.flush()?;

    let output = proc.exp_eof()?;
    let output = strip_until_alternate_screen_exit(&output);

    assert_eq!(output, "tere: Exited without changing folder\r\n");

    Ok(())
}

#[test]
fn first_run_prompt_if_history_file_doesnt_exist() -> Result<(), RexpectError> {
    let mut cmd = get_cmd();

    // set the XDG_CACHE_HOME to point to a temporary folder so that we always get the first run prompt
    let tmp = tempdir().expect("error creating temporary folder");
    cmd.env("XDG_CACHE_HOME", tmp.path().as_os_str());

    let mut proc = run_app_with_cmd(cmd);
    proc.send("n")?;
    proc.writer.flush()?;
    let output = proc.exp_eof()?;

    let ptn = Regex::new("It seems like you are running.*for the first time").unwrap();
    // check that first run prompt message is there
    assert!(ptn.find(&output).is_some());

    // check that having pressed 'n' prints the expected message
    assert_eq!(strip_until_alternate_screen_exit(&output), "Cancelled.");

    Ok(())
}
