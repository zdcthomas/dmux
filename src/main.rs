extern crate clap;
extern crate config;
extern crate dirs;

#[macro_use]
extern crate serde_derive;

extern crate grep_cli;
extern crate tmux_interface;
extern crate url;
extern crate walkdir;

mod select;
mod tmux;

use clap::{
    crate_authors, crate_description, crate_name, crate_version, value_t, values_t, App, Arg,
    SubCommand,
};
use regex::Regex;
use select::Selector;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tmux::{Layout, WorkSpace};
use url::Url;

fn args<'a>() -> clap::ArgMatches<'a> {
    App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("selected_dir")
                .help("Instead of opening the selector to pick a dir, open it is the desired dir.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("session_name")
                .short("s")
                .long("session_name")
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
            Arg::with_name("number_of_panes")
                .short("p")
                .long("panes")
                .help("number of panes")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("commands")
                .short("c")
                .multiple(true)
                .long("commands")
                .help("specify the window layout (layouts are dependent on the window number)")
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
            Arg::with_name("profile")
                .short("P")
                .long("profile")
                .help("Use a different configuration profile")
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

fn config_settings() -> config::Config {
    let default = Config::default();
    let mut settings = config::Config::default();
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
        .set_default("layout", default.layout)
        .unwrap()
        // the trait `std::convert::From<i32>` is not implemented for `config::value::ValueKind`
        .set_default("number_of_panes", default.number_of_panes as i64)
        .unwrap()
        .set_default("commands", default.commands)
        .unwrap()
        .set_default("session_name", default.session_name)
        .unwrap()
        .to_owned()
}

// I don't like the repetition here
#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default = "default_layout_checksum")]
    layout: String,
    #[serde(default = "default_session_name")]
    session_name: String,
    #[serde(default = "default_number_of_panes")]
    number_of_panes: i32,
    #[serde(default = "default_search_dir")]
    search_dir: PathBuf,
    #[serde(default = "default_commands")]
    commands: tmux::Commands,
}

fn default_search_dir() -> PathBuf {
    dirs::home_dir().unwrap()
}
fn default_layout_checksum() -> String {
    "34ed,230x56,0,0{132x56,0,0,3,97x56,133,0,222}".to_string()
}

fn default_session_name() -> String {
    "dev".to_string()
}

fn default_number_of_panes() -> i32 {
    2
}

fn default_commands() -> tmux::Commands {
    vec![String::from("vim"), String::from("ls")]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            layout: default_layout_checksum(),
            session_name: default_session_name(),
            number_of_panes: default_number_of_panes(),
            search_dir: dirs::home_dir().unwrap(),
            commands: default_commands(),
        }
    }
}

fn setup_workspace(selected_dir: PathBuf, config: Config) {
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

fn settings_config(settings: config::Config, target: Option<&str>) -> Config {
    if let Some(target) = target {
        let profile: Config = settings.get(target).unwrap();
        return profile;
    }
    let profile: Config = settings.try_into().unwrap();
    return profile;
}

fn main() {
    let settings = config_settings();
    let args = args();
    let conf_from_settings = settings_config(settings, args.value_of("profile"));

    let config = Config {
        session_name: value_t!(args.value_of("session_name"), String)
            .unwrap_or(conf_from_settings.session_name),
        layout: value_t!(args.value_of("layout"), String).unwrap_or(conf_from_settings.layout),
        number_of_panes: value_t!(args.value_of("number_of_panes"), i32)
            .unwrap_or(conf_from_settings.number_of_panes),
        commands: values_t!(args.values_of("commands"), String)
            .unwrap_or(conf_from_settings.commands),
        search_dir: value_t!(args.value_of("search_dir"), PathBuf)
            .unwrap_or(conf_from_settings.search_dir),
    };

    match args.subcommand_name() {
        None => open_in_selected_dir(args, config),

        Some("clone") => {
            let clone_args = args.subcommand_matches("clone").unwrap();
            let repo_url = clone_args
                .value_of("repo")
                .expect("No repo specified, what should I clone?");
            let dir: PathBuf;
            if let Some(t) = clone_args.value_of("target_dir") {
                let target_dir = PathBuf::from(t);
                dir = clone_from(repo_url, target_dir);
            } else {
                let target_dir = dirs::home_dir().unwrap();
                dir = clone_from(repo_url, target_dir);
            }
            setup_workspace(dir, config)
        }

        _ => unreachable!(),
    }
}

fn open_in_selected_dir(args: clap::ArgMatches, config: Config) {
    if let Ok(selected_dir) = value_t!(args.value_of("selected_dir"), PathBuf) {
        setup_workspace(selected_dir, config)
    } else if grep_cli::is_readable_stdin() {
        let selected_dir = PathBuf::from(read_line_iter());
        setup_workspace(selected_dir, config)
    } else if let Some(selected_dir) = Selector::new(config.search_dir.clone()).select_dir() {
        setup_workspace(selected_dir, config)
    }
}

fn read_line_iter() -> String {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().to_string()
}

// -> Result<Output, Error>
fn git_url_to_dir_name(url: &Url) -> String {
    let segments = url.path_segments().ok_or_else(|| "cannot be base").unwrap();
    let re = Regex::new(r"\.git$").unwrap();
    re.replace_all(segments.last().unwrap(), "").into_owned()
}

fn clone_from(repo: &str, target_dir: PathBuf) -> PathBuf {
    if let Ok(url) = Url::parse(repo) {
        let dir_name = git_url_to_dir_name(&url);
        let target = target_dir.join(dir_name);
        if !target.exists() {
            Command::new("git")
                .arg("clone")
                .arg(url.as_str())
                .arg(target.to_str().expect("couldn't make remote into dir"))
                .stdout(Stdio::inherit())
                .output()
                .expect("could not clone");
        }
        return target;
    }
    panic!("Sorry, {} isn't a valid url!", repo);
}

fn path_to_window_name(path: &Path) -> String {
    String::from(
        path.file_name()
            .expect("dir path contained invalid unicode")
            .to_str()
            .unwrap(),
    )
}
