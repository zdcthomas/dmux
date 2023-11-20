#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate anyhow;

mod app;
mod select;
mod tmux;

use anyhow::Result;
use app::CommandType;
use colored::*;
use select::Selector;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tmux::WorkSpace;
use url::Url;

fn main() {
    println!("test");
    
    if let Err(err) = run_command() {
        eprintln!("{}: {}", "Error".red(), err);
        err.chain()
            .skip(1)
            .for_each(|cause| eprintln!("because: {}", cause));
        std::process::exit(1);
    }
}

fn run_command() -> Result<()> {
    let command = app::build_app()?;

    if !tmux::has_tmux() {
        return Err(anyhow!("Tmux is not installed."));
    }
    match command {
        CommandType::Open(open_config) => open_selected_dir(open_config),
        CommandType::Select(select_config) => {
            match Selector::new(&select_config.workspace.search_dir).select_dir()? {
                Some(dir) => open_selected_dir(app::OpenArgs {
                    selected_dir: dir,
                    workspace: select_config.workspace,
                }),
                None => Ok(()),
            }
        }
        CommandType::Pull(pull_config) => match clone_from(&pull_config) {
            Ok(dir) => open_selected_dir(app::OpenArgs {
                selected_dir: dir,
                workspace: pull_config.workspace,
            }),
            Err(err) => Err(err),
        },
        CommandType::Layout => {
            if !tmux::in_tmux() {
                return Err(anyhow!("Not inside a tmux session. Run `tmux a` and select the window you want the layout of."));
            };
            tmux::generate_layout()
        }
    }
}

fn open_selected_dir(config: app::OpenArgs) -> Result<()> {
    if !config.selected_dir.exists() {
        return Err(anyhow!("{:?} isn't a valid path", config.selected_dir));
    }
    tmux::setup_workspace(WorkSpace {
        commands: config.workspace.commands,
        path: config.selected_dir,
        session_name: config.workspace.session_name,
        format_checksum: config.workspace.layout,
        window_name: config.workspace.window_name,
        number_of_panes: config.workspace.number_of_panes,
    });
    Ok(())
}

fn git_url_to_dir_name(git_url: &str) -> Result<String> {
    if let Ok(url) = Url::parse(git_url) {
        Ok(url
            .path_segments()
            .ok_or_else(|| anyhow!("cannot be base"))?
            .last()
            .ok_or_else(|| anyhow!("no segments"))?
            .replace(".git", ""))
    } else {
        Ok(git_url
            .split('/')
            .last()
            .ok_or_else(|| anyhow!("I don't know how to parse a dir from {:?}", git_url))?
            .replace(".git", ""))
    }
}

fn clone_from(config: &app::PullArgs) -> Result<PathBuf> {
    let dir_name = git_url_to_dir_name(&config.repo_url)?;
    let target = config.target_dir.join(dir_name);
    let output = Command::new("git")
        .arg("clone")
        .arg(config.repo_url.as_str())
        .arg(
            target
                .to_str()
                .ok_or_else(|| anyhow!("Specified target couldn't be used {:?}", target))?,
        )
        .stdout(Stdio::inherit())
        .output()?;
    if output.status.success() {
        Ok(target)
    } else {
        Err(anyhow!("{}", String::from_utf8(output.stderr)?))
    }
}

// fn path_to_string(path: &Path) -> Result<String> {
//     Ok(path
//         .to_str()
//         .ok_or_else(|| anyhow!("Invalid file"))?
//         .to_string())
// }

// fn path_to_window_name(path: &Path) -> Result<String> {
//     let file_str = path
//         .file_name()
//         .ok_or_else(|| anyhow!("No file name found"))?
//         .to_str()
//         .ok_or_else(|| anyhow!("Invalid file"));

//     Ok(String::from(file_str?))
// }

#[test]
fn git_url_to_dir_name_test() {
    assert_eq!(
        "dmux".to_string(),
        git_url_to_dir_name("https://github.com/zdcthomas/dmux").unwrap()
    );
    assert_eq!(
        "dmux".to_string(),
        git_url_to_dir_name("git@github.com:zdcthomas/dmux.git").unwrap()
    );
}
