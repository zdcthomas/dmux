// setup_workspace
// generate_layout
// in_tmux
// has_tmux

use std::cmp::max;
use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;
use tmux_interface::{NewWindow, TargetSession, TmuxCommand, TmuxOutput, Windows};

pub fn has_tmux() -> bool {
    tmux_exists("tmux")
}

fn tmux_exists(bin: &str) -> bool {
    std::process::Command::new(bin)
        .arg("-V")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn in_tmux() -> bool {
    is_tmux_value(std::env::var("TMUX").ok().as_deref())
}

fn is_tmux_value(value: Option<&str>) -> bool {
    // tmux always exports a socket path, so an empty value is somebody else's.
    matches!(value, Some(value) if !value.is_empty())
}

/// Turns a tmux command that ran but failed into an error.
///
/// `TmuxCommand::output` only reports the failures of the spawn itself, so
/// without this every tmux error is silent.
fn check(
    output: std::result::Result<TmuxOutput, tmux_interface::Error>,
    what: &str,
) -> Result<()> {
    let output = output.with_context(|| format!("could not run tmux to {}", what))?;
    if output.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.0.stderr).trim().to_string();
    if stderr.is_empty() {
        Err(anyhow!("tmux could not {}", what))
    } else {
        Err(anyhow!("tmux could not {}: {}", what, stderr))
    }
}

/// Reports a tmux failure without giving up on the workspace.
///
/// Use this only where the failure leaves a usable window, so that a bad
/// setting does not cost the user the whole workspace.
fn warn_on_failure(output: std::result::Result<TmuxOutput, tmux_interface::Error>, what: &str) {
    if let Err(err) = check(output, what) {
        eprintln!("{}: {}", "Warning".yellow(), err);
    }
}

/// `new-window` needs `-t` to say which session the window belongs to.
/// Without it tmux uses whichever session it considers current.
fn new_window_command<'a>(tmux: &TmuxCommand<'a>, workspace: &WorkSpace) -> Result<NewWindow<'a>> {
    let mut command = tmux.new_window();
    command
        .window_name(workspace.window_name()?)
        .start_directory(workspace.path_str()?)
        .target_window(workspace.session_target())
        .detached();
    Ok(command)
}

pub fn setup_workspace(workspace: WorkSpace) -> Result<()> {
    let tmux = TmuxCommand::new();
    let session_exists = tmux
        .has_session()
        .target_session(&workspace.session_name)
        .output()
        .with_context(|| {
            format!(
                "could not ask tmux about the session {:?}",
                workspace.session_name
            )
        })?
        .success();

    if session_exists {
        if window_exists(&workspace, &tmux)? {
            return attach_to_window(&workspace, &tmux);
        }
        check(
            new_window_command(&tmux, &workspace)?.output(),
            &format!("create the window {:?}", workspace.window_name()?),
        )?;
    } else {
        check(
            tmux.new_session()
                .session_name(&workspace.session_name)
                .start_directory(workspace.path_str()?)
                .detached()
                .window_name(workspace.window_name()?)
                .output(),
            &format!("create the session {:?}", workspace.session_name),
        )?;
    }

    // Creating either the session or the window already made one pane.
    setup_panes_with_commands(&workspace, &tmux)?;
    attach_to_window(&workspace, &tmux)
}

fn window_exists(workspace: &WorkSpace, _tmux: &TmuxCommand) -> Result<bool> {
    let target_session = TargetSession::Raw(&workspace.session_name);
    let wanted = workspace.window_name()?;
    let windows = Windows::get(&target_session, tmux_interface::WINDOW_ALL).map_err(|err| {
        anyhow!(
            "could not list the windows of the session {:?}: {}",
            workspace.session_name,
            err
        )
    })?;
    Ok(windows
        .into_iter()
        .any(|window| window.name.as_deref() == Some(wanted.as_str())))
}

fn setup_panes_with_commands(workspace: &WorkSpace, tmux: &TmuxCommand) -> Result<()> {
    let path = workspace.path_str()?;
    let window = workspace.window_target()?;

    // Counting up from 1 keeps the subtraction off a u8 that can be 0.
    for _ in 1..workspace.number_of_panes() {
        check(
            tmux.split_window()
                .start_directory(path.clone())
                .target_pane(window.clone())
                .output(),
            "split the window",
        )?;
    }

    // A layout that does not match the pane count is a bad setting, not a
    // reason to throw away a working window. tmux leaves the panes evenly
    // split, so warn and carry on.
    warn_on_failure(
        tmux.select_layout()
            .target_pane(workspace.pane_target(0)?)
            .layout_name(&workspace.format_checksum)
            .output(),
        &format!("apply the layout {:?}", workspace.format_checksum),
    );

    // A pane per command, and tmux cannot address more panes than a u8 holds.
    for (pane, command) in workspace
        .commands
        .iter()
        .enumerate()
        .take(usize::from(workspace.number_of_panes()))
    {
        let pane = pane as u8;
        check(
            tmux.send_keys()
                .target_pane(workspace.pane_target(pane)?)
                .key(format!("{}\r", command))
                .output(),
            &format!("send {:?} to pane {}", command, pane),
        )?;
    }
    Ok(())
}

fn attach_to_window(workspace: &WorkSpace, tmux: &TmuxCommand) -> Result<()> {
    let target = workspace.window_target()?;
    if in_tmux() {
        check(
            tmux.switch_client().target_session(target.clone()).output(),
            &format!("switch to the window {:?}", target),
        )
    } else {
        check(
            tmux.attach_session().target_session(target.clone()).output(),
            &format!("attach to the window {:?}", target),
        )
    }
}

pub fn generate_layout() -> Result<()> {
    let tmux = TmuxCommand::new();

    let stdout = tmux
        .list_windows()
        .format("#{window_active} #{window_layout}")
        .output()?
        .0
        .stdout;

    let layout = match std::str::from_utf8(&stdout)?
        .split('\n')
        .find(|l| l.starts_with('1'))
    {
        Some(layout) => Ok(layout),
        None => Err(anyhow!("Uh-oh, looks like you're not in a tmux session!")),
    }?;

    println!(
        "{}",
        layout
            .split_whitespace()
            .last()
            .ok_or_else(|| anyhow!("layout invalid"))?
    );
    Ok(())
}

#[derive(Debug, Clone)]
pub struct WorkSpace {
    pub path: PathBuf,
    pub session_name: String,
    pub format_checksum: String,
    pub commands: Vec<String>,
    pub window_name: Option<String>,
    pub number_of_panes: u8,
}

fn clean_str(string: &str) -> String {
    string.replace(['.', ' '], "-")
}

impl WorkSpace {
    /// The session on its own. The trailing colon is what makes tmux read this
    /// as a session rather than as a window called `session_name`.
    fn session_target(&self) -> String {
        format!("{}:", clean_str(&self.session_name))
    }

    fn window_target(&self) -> Result<String> {
        Ok(format!(
            "{}:{}",
            clean_str(&self.session_name),
            self.window_name()?
        ))
    }

    fn pane_target(&self, pane: u8) -> Result<String> {
        Ok(format!("{}.{}", self.window_target()?, pane))
    }

    fn window_name(&self) -> Result<String> {
        if let Some(name) = &self.window_name {
            // A dot here would make tmux read `sess:my.window.0` as window
            // `my`, pane `window`.
            return Ok(clean_str(name));
        }
        let file_name = self.path.file_name().ok_or_else(|| {
            anyhow!(
                "{:?} has no last component, so it cannot name a tmux window",
                self.path
            )
        })?;
        let file_name = file_name.to_str().ok_or_else(|| {
            anyhow!(
                "{:?} is not valid UTF-8, so it cannot name a tmux window",
                self.path
            )
        })?;
        Ok(clean_str(file_name))
    }

    fn path_str(&self) -> Result<String> {
        self.path
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("{:?} is not valid UTF-8", self.path))
    }

    fn number_of_panes(&self) -> u8 {
        let one_per_command = u8::try_from(self.commands.len()).unwrap_or(u8::MAX);
        max(1, max(one_per_command, self.number_of_panes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace() -> WorkSpace {
        WorkSpace {
            path: PathBuf::from("/Users/zacharythomas/dev/foo.bar/"),
            session_name: "dev".to_owned(),
            format_checksum: "34ed,230x56,0,0{132x56,0,0,3,97x56,133,0,222}".to_owned(),
            commands: vec!["nvim".to_owned(), "fish".to_owned()],
            window_name: None,
            number_of_panes: 3,
        }
    }

    #[test]
    fn clean_str_removes_dots_n_stuff() {
        assert_eq!(clean_str("foo.bar"), "foo-bar")
    }

    #[test]
    fn workplace_window_name_replaces_dots_n_spaces() {
        assert_eq!(workspace().window_name().unwrap(), "foo-bar")
    }

    #[test]
    fn workplace_window_name_returns_window_name_from_path() {
        let wp = WorkSpace {
            path: PathBuf::from("/Users/zacharythomas/dev/some_name/"),
            ..workspace()
        };
        assert_eq!(wp.window_name().unwrap(), "some_name")
    }

    #[test]
    fn window_name_cleans_an_explicit_window_name() {
        // An uncleaned dot makes tmux read `sess:my.window.0` as window `my`, pane `window`.
        let wp = WorkSpace {
            window_name: Some("my.window".to_owned()),
            ..workspace()
        };
        assert_eq!(wp.window_name().unwrap(), "my-window")
    }

    #[test]
    fn window_name_errors_when_the_path_has_no_file_name() {
        let wp = WorkSpace {
            path: PathBuf::from("/"),
            ..workspace()
        };
        assert!(wp.window_name().is_err())
    }

    #[test]
    fn path_str_errors_on_a_non_utf8_path() {
        use std::os::unix::ffi::OsStrExt;
        let wp = WorkSpace {
            path: PathBuf::from(std::ffi::OsStr::from_bytes(&[0xff, 0xfe])),
            ..workspace()
        };
        assert!(wp.path_str().is_err())
    }

    #[test]
    fn number_of_panes_is_never_zero() {
        // `0..n - 1` underflows a u8 and asks tmux for 255 panes.
        let wp = WorkSpace {
            commands: vec![],
            number_of_panes: 0,
            ..workspace()
        };
        assert_eq!(wp.number_of_panes(), 1)
    }

    #[test]
    fn number_of_panes_does_not_truncate_a_long_command_list() {
        let wp = WorkSpace {
            commands: vec!["ls".to_owned(); 300],
            number_of_panes: 2,
            ..workspace()
        };
        assert_eq!(wp.number_of_panes(), u8::MAX)
    }

    #[test]
    fn session_target_addresses_the_session_itself() {
        // `tmux new-window -t dev` is a window target, `-t dev:` is the session.
        assert_eq!(workspace().session_target(), "dev:")
    }

    #[test]
    fn window_target_names_the_session_and_the_window() {
        assert_eq!(workspace().window_target().unwrap(), "dev:foo-bar")
    }

    #[test]
    fn pane_target_names_the_session_the_window_and_the_pane() {
        assert_eq!(workspace().pane_target(2).unwrap(), "dev:foo-bar.2")
    }

    #[test]
    fn new_window_targets_the_configured_session() {
        let tmux = TmuxCommand::new();
        let command = new_window_command(&tmux, &workspace()).unwrap();
        let args: Vec<String> = command
            .0
            .cmd_args
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|a| a.to_string())
            .collect();

        let target = args
            .iter()
            .position(|a| a == "-t")
            .map(|i| args[i + 1].clone());
        assert_eq!(target, Some("dev:".to_owned()))
    }

    #[test]
    fn an_empty_tmux_variable_is_not_a_tmux_session() {
        // tmux always sets TMUX to a socket path, so an empty value means
        // something else exported it.
        assert!(!is_tmux_value(Some("")))
    }

    #[test]
    fn a_tmux_socket_path_is_a_tmux_session() {
        assert!(is_tmux_value(Some("/tmp/tmux-1000/default,123,0")))
    }

    #[test]
    fn an_unset_tmux_variable_is_not_a_tmux_session() {
        assert!(!is_tmux_value(None))
    }

    #[test]
    fn tmux_exists_is_false_for_a_missing_binary() {
        assert!(!tmux_exists("dmux-no-such-binary-4a7f"))
    }

    #[test]
    fn check_reports_the_stderr_of_a_failed_tmux_command() {
        use std::os::unix::process::ExitStatusExt;
        let output = TmuxOutput(std::process::Output {
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: Vec::new(),
            stderr: b"can't find session: dev".to_vec(),
        });

        let error = check(Ok(output), "attach to the session").unwrap_err();
        assert!(error.to_string().contains("can't find session: dev"))
    }

    #[test]
    fn check_passes_a_successful_tmux_command() {
        use std::os::unix::process::ExitStatusExt;
        let output = TmuxOutput(std::process::Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        });

        assert!(check(Ok(output), "attach to the session").is_ok())
    }
}
