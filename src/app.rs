#[path = "select.rs"]
pub mod select;

use clap::{
    crate_authors, crate_description, crate_name, crate_version, value_t, values_t, App, Arg,
    SubCommand,
};
use select::Selector;
use std::io;
use std::path::PathBuf;
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
            // use validator
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

// I don't like the repetition here
#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_layout_checksum")]
    pub layout: String,
    #[serde(default = "default_session_name")]
    pub session_name: String,
    #[serde(default = "default_number_of_panes")]
    pub number_of_panes: i32,
    #[serde(default = "default_search_dir")]
    pub search_dir: PathBuf,
    #[serde(default = "default_selected_dir")]
    pub selected_dir: PathBuf,
    #[serde(default = "default_commands")]
    pub commands: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            layout: default_layout_checksum(),
            session_name: default_session_name(),
            number_of_panes: default_number_of_panes(),
            search_dir: dirs::home_dir().unwrap(),
            selected_dir: dirs::home_dir().unwrap(),
            commands: default_commands(),
        }
    }
}

fn default_search_dir() -> PathBuf {
    dirs::home_dir().unwrap()
}

fn default_selected_dir() -> PathBuf {
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

fn default_commands() -> Vec<String> {
    vec!["vim".to_string(), "ls".to_string()]
}

fn config_file_settings() -> config::Config {
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

fn settings_config(settings: config::Config, target: Option<&str>) -> Config {
    if let Some(target) = target {
        let profile: Config = settings.get(target).unwrap();
        return profile;
    }
    let profile: Config = settings.try_into().unwrap();
    profile
}

pub struct PullConfig {
    pub repo_url: Url,
    pub target_dir: PathBuf,
    pub open_config: Config,
}

pub enum CommandType {
    Local(Config),
    Pull(PullConfig),
}

fn read_line_iter() -> String {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().to_string()
}

fn select_dir(args: &clap::ArgMatches, search_dir: PathBuf) -> PathBuf {
    if let Ok(selected_dir) = value_t!(args.value_of("selected_dir"), PathBuf) {
        selected_dir

    // selected_dir: value_t!(args.value_of("selected_dir"), PathBuf)
    //     .unwrap_or(conf_from_settings.selected_dir),
    } else if grep_cli::is_readable_stdin() && !grep_cli::is_tty_stdin() {
        PathBuf::from(read_line_iter())
    } else if let Some(selected_dir) = Selector::new(search_dir).select_dir() {
        selected_dir
    } else {
        panic!("something went very wrong");
    }
}

pub fn build_app() -> CommandType {
    let settings = config_file_settings();
    let args = args();
    let conf_from_settings = settings_config(settings, args.value_of("profile"));
    let search_dir =
        value_t!(args.value_of("search_dir"), PathBuf).unwrap_or(conf_from_settings.search_dir);
    let config = Config {
        session_name: value_t!(args.value_of("session_name"), String)
            .unwrap_or(conf_from_settings.session_name),
        layout: value_t!(args.value_of("layout"), String).unwrap_or(conf_from_settings.layout),
        number_of_panes: value_t!(args.value_of("number_of_panes"), i32)
            .unwrap_or(conf_from_settings.number_of_panes),
        commands: values_t!(args.values_of("commands"), String)
            .unwrap_or(conf_from_settings.commands),
        selected_dir: select_dir(&args, search_dir.clone()),
        search_dir,
    };

    match args.subcommand_name() {
        None => CommandType::Local(config),
        Some("clone") => {
            let clone_args = args.subcommand_matches("clone").unwrap();
            let url = clone_args
                .value_of("repo")
                .expect("No repo specified, what should I clone?");
            if let Ok(repo_url) = Url::parse(url) {
                let pull = PullConfig {
                    repo_url,
                    target_dir: value_t!(args.value_of("target_dir"), PathBuf)
                        .unwrap_or_else(|_| dirs::home_dir().unwrap()),
                    open_config: config,
                };
                CommandType::Pull(pull)
            } else {
                panic!("Sorry, {} isn't a valid url!", url);
            }
        }
        Some(_) => unreachable!("unexpected subcommand"),
    }
}
