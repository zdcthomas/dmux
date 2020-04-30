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
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tmux::{Layout, WorkSpace};
use url::Url;
// use std::env;

fn args<'a>() -> clap::ArgMatches<'a> {
    App::new("DMUX")
        .version("0.1.2")
        .author("Zdcthomas")
        .about("a nicer way to open up tmux 'workspaces'")
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

fn default_session_name() -> String {
    "dev".to_string()
}

#[derive(Deserialize, Default, Debug)]
struct Config {
    layout: String,
    session_name: String,
    number_of_panes: i32,
    search_dir: PathBuf,
    commands: tmux::Commands,
}

fn default_layout_checksum() -> String {
    "34ed,230x56,0,0{132x56,0,0,3,97x56,133,0,222}".to_string()
}

fn config_settings(settings: &mut config::Config) -> config::Config {
    let mut config_conf = dirs::config_dir().unwrap();
    config_conf.push("dmux/dmux.conf.xxxx");

    let mut home_conf = dirs::home_dir().unwrap();
    home_conf.push(".dmux.conf.xxxx");

    let mut mac_config = dirs::home_dir().unwrap();
    mac_config.push(".config/dmux/dmux.conf.xxx");
    settings
        // ~/dmux.conf.(yaml | json | toml)
        .merge(config::File::with_name(config_conf.to_str().unwrap()).required(false))
        .unwrap()
        // ~/{xdg_config|.config}dmux.conf.(yaml | json | toml)
        .merge(config::File::with_name(home_conf.to_str().unwrap()).required(false))
        .unwrap()
        .merge(config::File::with_name(mac_config.to_str().unwrap()).required(false))
        .unwrap()
        // Add in settings from the environment (with a prefix of DMUX)
        // Eg.. `DMUX_SESSION_NAME=foo dmux` would set the `session_name` key
        .merge(config::Environment::with_prefix("DMUX"))
        .unwrap()
        .to_owned()
}
fn default_commands() -> tmux::Commands {
    let mut commands = HashMap::new();
    commands.insert(0, String::from("vim"));
    commands.insert(1, String::from("ls -la"));
    commands
}

fn setup_workspace(selected_dir: String, config: Config) {
    // lol this is totally unnecessary now
    // it feels like I should just pass in a PathBuf
    let path = Path::new(selected_dir.as_str());
    let layout = Layout {
        layout_checksum: String::from(config.layout),
        window_count: config.number_of_panes,
    };

    let workspaces = WorkSpace {
        commands: config.commands,
        dir: String::from(path.to_str().unwrap()),
        layout,
        session_name: config.session_name,
        window_name: path_to_window_name(path).to_string(),
    };
    tmux::setup_workspace(workspaces);
}

fn main() {
    let settings = config_settings(&mut config::Config::default());
    let args = args();

    let config = Config {
        session_name: args
            .value_of("session")
            .unwrap_or(
                settings
                    .get::<String>("session_name")
                    .unwrap_or(default_session_name())
                    .as_str(),
            )
            .to_string(),
        layout: args
            .value_of("layout")
            .unwrap_or(
                settings
                    .get::<String>("layout")
                    .unwrap_or(default_layout_checksum())
                    .as_str(),
            )
            .to_string(),
        number_of_panes: args
            .value_of("number_of_panes")
            .unwrap_or(
                settings
                    // Ok Ok Ok yeah, I know, please tell me how to get ArgMatch::value_of to parse
                    // into a value and then I won't have to do this
                    .get::<i32>("number_of_panes")
                    .unwrap_or(2)
                    .to_string()
                    .as_str(),
            )
            .parse::<i32>()
            .expect("invalid number given"),
        // I don't know if it makes sense to have commands be a cli arg so right now, it's just
        // parsed from the config files/env
        commands: settings
            .get::<tmux::Commands>("commands")
            .unwrap_or(default_commands()),
        search_dir: dirs::home_dir().unwrap(),
    };

    match args.subcommand_name() {
        None => {
            if let Some(selected_dir) = Selector::new(&config.search_dir).select_dir() {
                setup_workspace(selected_dir, config)
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
            setup_workspace(dir, config)
        }

        _ => unreachable!(),
    }
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

fn path_to_window_name(path: &Path) -> String {
    String::from(
        path.file_name()
            .expect("dir path contained invalid unicode")
            .to_str()
            .unwrap(),
    )
}
