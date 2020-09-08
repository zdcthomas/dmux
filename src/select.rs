use anyhow::Result;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use walkdir::{DirEntry, WalkDir};

fn is_git_dir(entry: &DirEntry) -> bool {
    if let Some(file_name) = entry.file_name().to_str() {
        file_name.contains(&"git")
    } else {
        false
    }
}

fn all_dirs_in_path(search_dir: &PathBuf) -> String {
    let mut path_input = String::new();
    for entry in WalkDir::new(search_dir)
        .max_depth(4)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir() && !is_git_dir(e))
    {
        if let Ok(path) = entry {
            path_input.push_str("\n");
            path_input.push_str(path.path().to_str().unwrap());
        }
    }
    path_input
}

pub struct Selector {
    search_dir: PathBuf,
    use_fd: bool,
}

fn output_to_pathbuf(output: Output) -> Option<PathBuf> {
    if output.status.success() {
        let mut stdout = output.stdout;
        // removes trailing newline, probably a better way to do this
        stdout.pop();
        let path: PathBuf = String::from_utf8(stdout).unwrap().parse().unwrap();
        Some(path)
    } else {
        // Err(anyhow!("Couldn't parse path from {:?}", output.stdout))
        None
    }
}

impl Selector {
    pub fn new(search_dir: &PathBuf) -> Selector {
        let use_fd = Command::new("fd")
            .arg("--version")
            .stdout(Stdio::null())
            .spawn()
            .is_ok();
        Selector {
            search_dir: search_dir.to_owned(),
            use_fd,
        }
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
        let fzf = Command::new("fzf-tmux")
            .stdin(pipe)
            .stdout(Stdio::piped())
            .spawn()?;
        let output = fzf.wait_with_output()?;
        fd.kill()?;
        Ok(output_to_pathbuf(output))
    }

    fn select_with_walk_dir(&self) -> Result<Option<PathBuf>> {
        let files = all_dirs_in_path(&self.search_dir);
        let mut fzf = Command::new("fzf-tmux")
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

        Ok(output_to_pathbuf(output))
    }

    pub fn select_dir(&self) -> Result<Option<PathBuf>> {
        if self.use_fd {
            self.select_with_fd()
        } else {
            self.select_with_walk_dir()
        }
    }
}
