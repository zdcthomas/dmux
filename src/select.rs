use anyhow::Result;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use walkdir::{DirEntry, WalkDir};

fn is_git_dir(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(is_git_dir_name)
        .unwrap_or(false)
}

fn is_git_dir_name(name: &str) -> bool {
    name == ".git"
}

/// Looks for a binary on `PATH` instead of running it. Running it would leave
/// a zombie behind, and `fzf-tmux --version` can try to open a pane.
fn binary_exists(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| dir.join(bin).is_file()))
        .unwrap_or(false)
}

/// `fzf-tmux` only earns its keep inside tmux, and some packages ship `fzf`
/// without it.
fn selector_binary(in_tmux: bool, has_fzf_tmux: bool, has_fzf: bool) -> Option<&'static str> {
    if in_tmux && has_fzf_tmux {
        Some("fzf-tmux")
    } else if has_fzf {
        Some("fzf")
    } else {
        None
    }
}

fn parse_selection(stdout: &[u8]) -> Option<PathBuf> {
    let selection = std::str::from_utf8(stdout).ok()?.trim_end_matches(['\n', '\r']);
    if selection.is_empty() {
        return None;
    }
    Some(PathBuf::from(selection))
}

fn all_dirs_in_path(search_dir: &PathBuf) -> String {
    let mut path_input = String::new();
    for entry in WalkDir::new(search_dir)
        .max_depth(4)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir() && !is_git_dir(e))
        .flatten()
    {
        if let Some(path) = entry.path().to_str() {
            path_input.push('\n');
            path_input.push_str(path);
        }
    }
    path_input
}

pub struct Selector {
    search_dir: PathBuf,
    use_fd: bool,
}

impl Selector {
    pub fn new(search_dir: &PathBuf) -> Selector {
        Selector {
            search_dir: search_dir.to_owned(),
            use_fd: binary_exists("fd"),
        }
    }

    fn selector_command(&self) -> Result<Command> {
        let binary = selector_binary(
            crate::tmux::in_tmux(),
            binary_exists("fzf-tmux"),
            binary_exists("fzf"),
        )
        .ok_or_else(|| {
            anyhow!("dmux needs fzf to select a dir. Install fzf, or pass a path to dmux.")
        })?;
        Ok(Command::new(binary))
    }

    fn select_with_fd(&self) -> Result<Option<PathBuf>> {
        let mut fd = Command::new("fd")
            .arg("-td")
            .arg(".")
            .arg(
                self.search_dir
                    .to_str()
                    .ok_or_else(|| anyhow!("couldn't make search dir a string"))?,
            )
            .stdout(Stdio::piped())
            .spawn()?;

        let pipe = fd
            .stdout
            .take()
            .ok_or_else(|| anyhow!("FD command's stdout could not be read"))?;
        let fzf = self
            .selector_command()?
            .stdin(pipe)
            .stdout(Stdio::piped())
            .spawn()?;
        let output = fzf.wait_with_output()?;
        fd.kill()?;
        fd.wait()?;
        if !output.status.success() {
            return Ok(None);
        }
        Ok(parse_selection(&output.stdout))
    }

    fn select_with_walk_dir(&self) -> Result<Option<PathBuf>> {
        let files = all_dirs_in_path(&self.search_dir);
        let mut fzf = self
            .selector_command()?
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        // this should be converted to an async stream so that
        // selection doesn't have to wait for dir traversal
        fzf.stdin
            .as_mut()
            .ok_or_else(|| anyhow!("fzf couldn't take stdin"))?
            .write_all(files.as_bytes())?;

        let output = fzf.wait_with_output()?;
        if !output.status.success() {
            return Ok(None);
        }
        Ok(parse_selection(&output.stdout))
    }

    pub fn select_dir(&self) -> Result<Option<PathBuf>> {
        if self.use_fd {
            self.select_with_fd()
        } else {
            self.select_with_walk_dir()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dot_git_dir_is_skipped() {
        assert!(is_git_dir_name(".git"))
    }

    #[test]
    fn a_dir_that_merely_contains_git_is_not_skipped() {
        // Skipping these removed whole subtrees from the selector.
        assert!(!is_git_dir_name("github"));
        assert!(!is_git_dir_name("gitlab"));
        assert!(!is_git_dir_name("digit"));
        assert!(!is_git_dir_name("my-git-tools"));
    }

    #[test]
    fn fzf_tmux_is_preferred_inside_tmux() {
        assert_eq!(selector_binary(true, true, true), Some("fzf-tmux"))
    }

    #[test]
    fn plain_fzf_is_used_outside_tmux() {
        assert_eq!(selector_binary(false, true, true), Some("fzf"))
    }

    #[test]
    fn plain_fzf_is_used_when_the_tmux_wrapper_is_missing() {
        // Some packages install fzf without fzf-tmux.
        assert_eq!(selector_binary(true, false, true), Some("fzf"))
    }

    #[test]
    fn there_is_no_selector_without_fzf() {
        assert_eq!(selector_binary(true, false, false), None)
    }

    #[test]
    fn a_selection_loses_its_trailing_newline() {
        assert_eq!(
            parse_selection(b"/home/zach/dev\n"),
            Some(PathBuf::from("/home/zach/dev"))
        )
    }

    #[test]
    fn a_selection_without_a_trailing_newline_is_kept_whole() {
        assert_eq!(
            parse_selection(b"/home/zach/dev"),
            Some(PathBuf::from("/home/zach/dev"))
        )
    }

    #[test]
    fn an_empty_selection_is_no_selection() {
        assert_eq!(parse_selection(b""), None)
    }

    #[test]
    fn a_blank_selection_is_no_selection() {
        assert_eq!(parse_selection(b"\n"), None)
    }

    #[test]
    fn a_non_utf8_selection_is_no_selection() {
        assert_eq!(parse_selection(&[0xff, 0xfe]), None)
    }

    #[test]
    fn binary_exists_is_false_for_a_missing_binary() {
        assert!(!binary_exists("dmux-no-such-binary-4a7f"))
    }

    #[test]
    fn binary_exists_finds_a_binary_on_the_path() {
        assert!(binary_exists("sh"))
    }
}
