#[macro_use]
extern crate serde_derive;
extern crate clap;
extern crate config;
extern crate dirs;
extern crate grep_cli;
extern crate skim;
extern crate tmux_interface;
extern crate url;
extern crate walkdir;

mod app;
mod select;
mod tmux;

use app::CommandType;
use select::Selector;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tmux::{Layout, WorkSpace};
use url::Url;

fn main() {
    let command = app::build_app();

    match command {
        CommandType::Open(open_config) => open_selected_dir(open_config),
        CommandType::Select(select_config) => {
            if let Some(dir) = Selector::new(&select_config.workspace.search_dir).select_dir() {
                open_selected_dir(app::OpenArgs {
                    selected_dir: dir,
                    workspace: select_config.workspace,
                })
            }
        }
        CommandType::Pull(pull_config) => {
            let dir = clone_from(&pull_config);
            open_selected_dir(app::OpenArgs {
                selected_dir: dir,
                workspace: pull_config.workspace,
            })
        }
        CommandType::Layout => {
            if !tmux::in_tmux() {
                panic!("Not inside a tmux session. Run `tmux a` and select the window you want the layout of.")
            }
            tmux::generate_layout()
        }
    }
}

fn open_selected_dir(config: app::OpenArgs) {
    if !config.selected_dir.exists() {
        panic!("dude, that's not a path")
    }
    let layout = Layout {
        layout_string: config.workspace.layout,
        window_count: config.workspace.number_of_panes,
    };
    let workspaces = WorkSpace {
        commands: config.workspace.commands,
        dir: String::from(config.selected_dir.to_str().unwrap()),
        layout,
        session_name: config.workspace.session_name,
        window_name: path_to_window_name(&config.selected_dir).to_string(),
    };
    tmux::setup_workspace(workspaces);
}

// TODO: -> Result<Output, Error>
fn git_url_to_dir_name(url: &Url) -> String {
    let segments = url.path_segments().ok_or_else(|| "cannot be base").unwrap();
    segments.last().unwrap().replace(".git", "")
}

fn clone_from(config: &app::PullArgs) -> PathBuf {
    let dir_name = git_url_to_dir_name(&config.repo_url);
    let target = config.target_dir.join(dir_name);
    if !target.exists() {
        Command::new("git")
            .arg("clone")
            .arg(config.repo_url.as_str())
            .arg(target.to_str().expect("couldn't make remote into dir"))
            .stdout(Stdio::inherit())
            .output()
            .expect("could not clone");
    }
    target
}

fn path_to_window_name(path: &Path) -> String {
    String::from(
        path.file_name()
            .expect("dir path contained invalid unicode")
            .to_str()
            .unwrap(),
    )
}
