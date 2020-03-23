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

fn all_dirs_in_path(search_dir: PathBuf) -> String {
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
    files: String,
}

fn output_to_string(output: Output) -> Option<String> {
    if output.status.success() {
        let mut stdout = String::from_utf8(output.stdout).unwrap();
        stdout.pop();
        return Some(stdout);
    } else {
        return None;
    }
}

impl Selector {
    pub fn new(search_dir: PathBuf) -> Selector {
        let files = all_dirs_in_path(search_dir);
        return Selector { files: files };
    }

    pub fn select_dir(&self) -> Option<String> {
        let mut fzf = Command::new("fzf")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        fzf.stdin
            .as_mut()
            .unwrap()
            .write_all(self.files.as_bytes())
            .unwrap();

        let output = fzf.wait_with_output().unwrap();
        return output_to_string(output);
    }
}
