use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

fn is_git_dir(entry: &DirEntry) -> bool {
    if let Some(file_name) = entry.file_name().to_str() {
        return file_name.contains(&"git");
    } else {
        return false;
    }
}

pub fn all_dirs_in_path(search_dir: PathBuf) -> String {
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

// Note, i really want to just go and steal what i need from fd
