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
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tmux::WorkSpace;
use url::Url;

fn main() {
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
    })
}

fn git_url_to_dir_name(git_url: &str) -> Result<String> {
    let last_segment = if let Ok(url) = Url::parse(git_url) {
        url.path_segments()
            .ok_or_else(|| anyhow!("cannot be base"))?
            .last()
            .ok_or_else(|| anyhow!("no segments"))?
            .to_owned()
    } else {
        git_url
            .split('/')
            .last()
            .ok_or_else(|| anyhow!("I don't know how to parse a dir from {:?}", git_url))?
            .to_owned()
    };
    // Only the suffix: a repo called `bar.github` keeps its name.
    Ok(last_segment
        .strip_suffix(".git")
        .unwrap_or(&last_segment)
        .to_owned())
}

/// Where a clone lands. `--name` wins over the name in the URL.
fn clone_target(base: &Path, name: Option<&str>, repo_url: &str) -> Result<PathBuf> {
    let name = match name {
        Some(name) => name.to_owned(),
        None => git_url_to_dir_name(repo_url)?,
    };
    // A single plain component, so that a name cannot reach outside base.
    let mut components = Path::new(&name).components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(_)), None) => Ok(base.join(name)),
        _ => Err(anyhow!("{:?} is not a usable directory name for a clone", name)),
    }
}

fn clone_from(config: &app::PullArgs) -> Result<PathBuf> {
    let target = clone_target(
        &config.target_dir,
        config.name.as_deref(),
        &config.repo_url,
    )?;
    // Inherit both streams, so that the user sees git's progress on a slow
    // clone. `output()` would capture the progress and show nothing.
    let status = Command::new("git")
        .arg("clone")
        .arg(config.repo_url.as_str())
        .arg(
            target
                .to_str()
                .ok_or_else(|| anyhow!("Specified target couldn't be used {:?}", target))?,
        )
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if status.success() {
        Ok(target)
    } else {
        Err(anyhow!("git could not clone {:?}", config.repo_url))
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn only_a_trailing_dot_git_is_stripped() {
        // `.replace` removed the substring wherever it appeared.
        assert_eq!(
            "bar.github".to_string(),
            git_url_to_dir_name("https://github.com/foo/bar.github").unwrap()
        );
        assert_eq!(
            "my.gitlab.thing".to_string(),
            git_url_to_dir_name("git@example.com:foo/my.gitlab.thing.git").unwrap()
        );
    }

    #[test]
    fn a_clone_without_a_name_uses_the_name_from_the_url() {
        assert_eq!(
            clone_target(Path::new("/tmp/repos"), None, "https://github.com/foo/bar.git").unwrap(),
            PathBuf::from("/tmp/repos/bar")
        );
    }

    #[test]
    fn a_clone_with_a_name_uses_that_name() {
        assert_eq!(
            clone_target(
                Path::new("/tmp/repos"),
                Some("renamed"),
                "https://github.com/foo/bar.git"
            )
            .unwrap(),
            PathBuf::from("/tmp/repos/renamed")
        );
    }

    #[test]
    fn a_clone_name_cannot_escape_the_target_dir() {
        assert!(clone_target(Path::new("/tmp/repos"), Some("../etc"), "https://x/y.git").is_err());
    }
}
