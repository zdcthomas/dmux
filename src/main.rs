extern crate clap;
extern crate config;
extern crate dirs;

#[macro_use]
extern crate serde_derive;

extern crate grep_cli;
extern crate skim;
extern crate tmux_interface;
extern crate url;
extern crate walkdir;

mod app;
// if this isn't pub the compiler yells at me about dead code, which confuses me greatly
mod select;
mod tmux;

use app::{
    CommandType::{Local, Pull},
    Config,
};
use select::Selector;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tmux::{Layout, WorkSpace};
use url::Url;

fn setup_workspace(config: Config, maybe_dir: Option<PathBuf>) {
    let selected_dir: PathBuf;
    if let Some(dir) = maybe_dir {
        selected_dir = dir;
    } else if let Some(dir) = config.selected_dir {
        selected_dir = dir;
    } else {
        panic!("something went super wrong");
    }

    // TODO: can i get rid of this? maybe panic earlier
    if !selected_dir.exists() {
        panic!("dude, that's not a path")
    }
    let layout = Layout {
        layout_string: config.layout,
        window_count: config.number_of_panes,
    };
    let workspaces = WorkSpace {
        commands: config.commands,
        dir: String::from(selected_dir.to_str().unwrap()),
        layout,
        session_name: config.session_name,
        window_name: path_to_window_name(&selected_dir).to_string(),
    };
    tmux::setup_workspace(workspaces);
}

fn main() {
    let command = app::build_app();

    match command {
        Local(config) => {
            if config.selected_dir.is_some() {
                setup_workspace(config, None)
            } else if let Some(dir) = Selector::new(&config.search_dir).select_dir() {
                setup_workspace(config, Some(dir))
            } else {
                panic!()
            }
        }

        Pull(pull_config) => {
            let dir = clone_from(&pull_config);
            setup_workspace(pull_config.open_config, Some(dir))
        }
    }
}

// TODO: -> Result<Output, Error>
fn git_url_to_dir_name(url: &Url) -> String {
    let segments = url.path_segments().ok_or_else(|| "cannot be base").unwrap();
    // TODO: use str.replace here
    // let re = Regex::new(r"\.git$").unwrap();
    // re.replace_all(segments.last().unwrap(), "").into_owned()
    segments.last().unwrap().replace(".git", "")
}

fn clone_from(config: &app::PullConfig) -> PathBuf {
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
