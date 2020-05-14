use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use walkdir::{DirEntry, WalkDir};

fn is_git_dir(entry: &DirEntry) -> bool {
    if let Some(file_name) = entry.file_name().to_str() {
        return file_name.contains(&"git");
    } else {
        return false;
    }
}

fn all_dirs_in_path(search_dir: &PathBuf) -> String {
    // let home = dirs::home_dir().unwrap();
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
    return path_input;
}

// SELECTOR
pub struct Selector {
    search_dir: PathBuf,
    use_fd: bool,
}

fn output_to_pathbuf(output: Output) -> Option<PathBuf> {
    if output.status.success() {
        let mut stdout = output.stdout;
        // removes trailing newline
        stdout.pop();
        let path: PathBuf = String::from_utf8(stdout).unwrap().parse().unwrap();
        return Some(path);
    } else {
        return None;
    }
}

impl Selector {
    pub fn new(search_dir: PathBuf) -> Selector {
        let mut use_fd = false;
        if let Ok(_) = Command::new("fd")
            .arg("--version")
            .stdout(Stdio::null())
            .spawn()
        {
            use_fd = true;
        }
        return Selector { search_dir, use_fd };
    }

    fn select_with_fd(&self) -> Option<PathBuf> {
        let mut fd = Command::new("fd")
            .arg("-td")
            .arg(".")
            .arg(
                self.search_dir
                    .to_str()
                    .expect("couldn't make search dir a string"),
            )
            .stdout(Stdio::piped())
            .spawn()
            .expect("fd failed unexpectedly");

        let pipe = fd.stdout.take().unwrap();
        let fzf = Command::new("fzf-tmux")
            .stdin(pipe)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let output = fzf.wait_with_output().expect("fzf failed unexpectedly");
        fd.kill().expect("could not kill fd process");
        return output_to_pathbuf(output);
    }

    fn select_with_walk_dir(&self) -> Option<PathBuf> {
        let files = all_dirs_in_path(&self.search_dir);
        let mut fzf = Command::new("fzf-tmux")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        // this should be converted to an async stream so that
        // selection doesn't have to wait for dir traversal
        fzf.stdin
            .as_mut()
            .unwrap()
            .write_all(files.as_bytes())
            .unwrap();

        let output = fzf.wait_with_output().unwrap();

        return output_to_pathbuf(output);
    }

    pub fn select_dir(&self) -> Option<PathBuf> {
        if self.use_fd {
            self.select_with_fd()
        } else {
            self.select_with_walk_dir()
        }
    }
}
