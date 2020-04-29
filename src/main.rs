extern crate clap;
extern crate config;
extern crate dirs;
extern crate interactor;

#[macro_use]
extern crate serde_derive;

extern crate tmux_interface;
extern crate url;
extern crate walkdir;

mod select;
mod tmux;

use clap::{App, Arg, SubCommand};
use regex::Regex;
use select::Selector;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use tmux::{Layout, WorkSpace};
use url::Url;
// use std::env;

fn default_session_name<'a>() -> &'a str {
    return "dev";
}
// -> Result<Output, Error>
fn git_url_to_dir_name(url: &Url) -> String {
    let segments = url.path_segments().ok_or_else(|| "cannot be base").unwrap();
    let re = Regex::new(r"\.git$").unwrap();
    re.replace_all(segments.last().unwrap(), "").into_owned()
}

fn clone_from(repo: &str, target_dir: &Path) -> String {
    if let Ok(url) = Url::parse(repo) {
        let dir_name = git_url_to_dir_name(&url);
        let target = target_dir.clone().join(dir_name.clone());
        let target_string = target.to_str().expect("couldn't make remote into dir");
        if !target.exists() {
            Command::new("git")
                .arg("clone")
                .arg(url.as_str())
                .arg(target_string)
                .stdout(Stdio::inherit())
                .output()
                .expect("could not clone");
        } else {
        }
        return target_string.to_owned();
    } else {
        panic!("Ooopsie Whoopsie, {} isn't a valid url!", repo);
    }
}

fn path_to_window_name(path: &Path) -> &str {
    path.file_name()
        .expect("dir path contained invalid unicode")
        .to_str()
        .unwrap()
}

#[derive(Deserialize)]
struct Config {
    layout: Option<String>,
    session_name: Option<String>,
    number_of_panes: Option<i32>,
}

fn main() {
    let mut settings = config::Config::default();

    let mut search_dir = dirs::home_dir().unwrap();
    search_dir.push("config/dmux/conf");
    println!("{:?}", search_dir.to_str());
    settings
        // Add in `./Settings.toml`
        .merge(config::File::from(search_dir))
        .unwrap()
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .merge(config::Environment::with_prefix("DMUX"))
        .unwrap();
    let foo = settings.try_into::<Config>().unwrap();
    println!("{:?}", foo.layout);
    println!("{:?}", foo.session_name);

    let args = args();

    let session_name = args.value_of("session").unwrap_or(default_session_name());

    let number_of_panes = args
        .value_of("number_of_panes")
        .unwrap_or("2")
        .parse::<i32>()
        .expect("provided number of panes not a valid number");

    let layout = args
        .value_of("layout")
        .unwrap_or(tmux::default_layout_checksum());

    let search_dir = dirs::home_dir().unwrap();

    match args.subcommand_name() {
        None => {
            if let Some(selected_dir) = Selector::new(search_dir).select_dir() {
                setup_workspace(selected_dir, number_of_panes, layout, session_name)
            }
        }

        Some("clone") => {
            let clone = args.subcommand_matches("clone").unwrap();
            let repo = clone
                .value_of("repo")
                .expect("No repo specified, what should I clone?");
            let dir: String;
            if let Some(t) = clone.value_of("target_dir") {
                let target_dir = Path::new(t);
                dir = clone_from(repo, &target_dir);
            } else {
                let target_dir = dirs::home_dir().unwrap();
                dir = clone_from(repo, &target_dir);
            }
            setup_workspace(dir, number_of_panes, layout, session_name)
        }

        _ => unreachable!(),
    }
}

fn setup_workspace(selected_dir: String, number_of_panes: i32, layout: &str, session_name: &str) {
    let path = Path::new(selected_dir.as_str());
    let layout = Layout {
        window_count: number_of_panes,
        layout_checksum: String::from(layout),
    };

    let mut commands = HashMap::new();
    commands.insert(0, String::from("nvim"));
    commands.insert(1, String::from("fish"));

    let workspaces = WorkSpace {
        session_name,
        window_name: path_to_window_name(path),
        dir: path.to_str().expect("oops on path str"),
        layout,
        commands,
    };
    tmux::setup_workspace(workspaces);
}

fn args<'a>() -> clap::ArgMatches<'a> {
    App::new("DMUX")
        .version("0.1.2")
        .author("Zdcthomas")
        .about("a nicer way to open up tmux 'workspaces'")
        .arg(Arg::with_name("repo").help("clones a repo from a git remote"))
        .arg(
            Arg::with_name("session_name")
                .short("s")
                .long("session")
                .help("specify a specific session name to run")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("window_name")
                .short("w")
                .long("window")
                .help("specify the window name")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("layout")
                .short("l")
                .long("layout")
                .help("specify the window layout (layouts are dependent on the window number)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("search_dir")
                .short("d")
                .long("dir")
                .help("override of the dir to select from")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("clone")
                .about("clones a git repository")
                .arg(Arg::with_name("repo").help("specifies the repo to clone from"))
                .arg(
                    Arg::with_name("name")
                        .short("n")
                        .long("name")
                        .help("sets the local name for the cloned repo")
                        .takes_value(true),
                ),
        )
        .get_matches()
}
