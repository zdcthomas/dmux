// setup_workspace
// generate_layout
// in_tmux
// has_tmux

use std::cmp::max;
use std::path::PathBuf;

use anyhow::Result;
use tmux_interface::{TargetSession, TmuxCommand, Windows};

pub fn has_tmux() -> bool {
    std::process::Command::new("tmux")
        .arg("-V")
        .output()
        .unwrap()
        .status
        .success()
}

pub fn in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

pub fn setup_workspace(workspace: WorkSpace) {
    let tmux = TmuxCommand::new();
    let session_with_right_name_exists = tmux
        .has_session()
        .target_session(&workspace.session_name)
        .output()
        .unwrap()
        .success();

    if session_with_right_name_exists {
        let target_session = TargetSession::Raw(&workspace.session_name);
        let window_with_right_name_exists =
            Windows::get(&target_session, tmux_interface::WINDOW_ALL)
                .unwrap()
                .into_iter()
                .any(|w| w.name.as_ref().unwrap() == &workspace.window_name());

        if window_with_right_name_exists {
            attach_to_window(&workspace, &tmux);
        } else {
            // create window
            tmux.new_window()
                .window_name(workspace.window_name())
                .start_directory(workspace.path_str())
                // first command goes in defaut pane
                .detached()
                .output()
                .unwrap();

            // one already exists from when the window was created
            setup_panes_with_commands(&workspace, &tmux);

            attach_to_window(&workspace, &tmux);
        };
    } else {
        // No existing tmux session

        // Create a new session
        tmux.new_session()
            .session_name(&workspace.session_name)
            .start_directory(&workspace.path_str())
            .detached()
            .window_name(workspace.window_name())
            .output()
            .unwrap();

        setup_panes_with_commands(&workspace, &tmux);

        attach_to_window(&workspace, &tmux);
    };
}

fn setup_panes_with_commands(workspace: &WorkSpace, tmux: &TmuxCommand) {
    for _ in 0..workspace.number_of_panes() - 1 {
        tmux.split_window()
            .start_directory(workspace.path_str())
            .target_pane(workspace.target_session(None))
            .output()
            .unwrap();
    }

    tmux.select_layout()
        .target_pane(workspace.target_session(Some(0)))
        .layout_name(&workspace.format_checksum)
        .output()
        .unwrap();

    workspace.commands.iter().enumerate().for_each(|(i, com)| {
        tmux.send_keys()
            .target_pane(workspace.target_session(Some(i as u8)))
            .key(format!("{}\r", com))
            .output()
            .unwrap();
    });
}

fn attach_to_window(workspace: &WorkSpace, tmux: &TmuxCommand) {
    if in_tmux() {
        // switch to the window which exists
        tmux.switch_client()
            .target_session(workspace.target_session(None))
            .output()
            .unwrap();
    } else {
        // attach to the window in the session
        tmux.attach_session()
            .target_session(workspace.target_session(None))
            .output()
            .unwrap();
    };
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
    string.replace(".", "-").replace(" ", "-")
}

impl WorkSpace {
    fn target_session(&self, pane: Option<u8>) -> String {
        if let Some(pane) = pane {
            format!(
                "{}:{}.{}",
                clean_str(&self.session_name),
                self.window_name(),
                pane
            )
        } else {
            format!("{}:{}", clean_str(&self.session_name), self.window_name())
        }
    }

    fn window_name(&self) -> String {
        if let Some(name) = &self.window_name {
            name.to_owned()
        } else {
            clean_str(
                &self
                    .path
                    .file_name()
                    .unwrap()
                    .to_owned()
                    .into_string()
                    .unwrap(),
            )
        }
    }

    fn path_str(&self) -> String {
        self.path.as_os_str().to_owned().into_string().unwrap()
    }

    fn number_of_panes(&self) -> u8 {
        max(self.commands.len() as u8, self.number_of_panes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clean_str_removes_dots_n_stuff() {
        assert_eq!(clean_str("foo.bar"), "foo-bar")
    }

    #[test]
    fn workplace_window_name_replaces_dots_n_spaces() {
        let wp = WorkSpace {
            path: PathBuf::from("/Users/zacharythomas/dev/foo.bar/"),
            session_name: "dev".to_owned(),
            format_checksum: "34ed,230x56,0,0{132x56,0,0,3,97x56,133,0,222}".to_owned(),
            commands: vec!["nvim".to_owned(), "fish".to_owned()],
            window_name: None,
            number_of_panes: 3,
        };
        assert_eq!(wp.window_name(), "foo-bar")
    }

    #[test]
    fn workplace_window_name_returns_window_name_from_path() {
        let wp = WorkSpace {
            path: PathBuf::from("/Users/zacharythomas/dev/some_name/"),
            session_name: "dev".to_owned(),
            format_checksum: "34ed,230x56,0,0{132x56,0,0,3,97x56,133,0,222}".to_owned(),
            commands: vec!["nvim".to_owned(), "fish".to_owned()],
            window_name: None,
            number_of_panes: 3,
        };
        assert_eq!(wp.window_name(), "some_name")
    }
}
