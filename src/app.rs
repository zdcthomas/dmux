use anyhow::Result;
use clap::{
    crate_authors, crate_description, crate_name, crate_version, value_t, values_t, App, Arg,
    SubCommand,
};
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn args<'a>() -> clap::ArgMatches<'a> {
    let fzf_available = Command::new("fzf")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok();
    App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("selected_dir")
                .help("Open this directory directly without starting a selector")
                .takes_value(true)
                // if fzf isn't available, this needs to be specified
                .required(!fzf_available),
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
                .help("the number of panes to generate.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("commands")
                .short("c")
                .multiple(true)
                .long("commands")
                .help("commands to run in panes")
                .long_help(commands_long_help().as_str())
                .takes_value(true),
        )
        .arg(
            // We should use validator here
            Arg::with_name("layout")
                .short("l")
                .long("layout")
                .help("specify the window layout (layouts are dependent on the number of panes)")
                .long_help(layout_long_help().as_str())
                .takes_value(true),
        )
        .arg(
            Arg::with_name("profile_from_dir")
                .short("D")
                .help("Take the configuration file from the selected directory.")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("profile")
                .short("P")
                .long("profile")
                .help("Use a different configuration profile.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("search_dir")
                .short("d")
                .long("dir")
                .help("override of the dir to select from.")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("clone")
                .about("clones a git repository, and then opens a workspace in the repo")
                .arg(
                    Arg::with_name("repo")
                        .help("specifies the repo to clone from")
                        .required(true),
                )
                .arg(
                    Arg::with_name("name")
                        .short("n")
                        .long("name")
                        .help("sets the local name for the cloned repo")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("layout").about("generates the current layout string from tmux"),
        )
        .get_matches()
}

fn layout_long_help() -> String {
    format!(
        "This string is the same representation that
tmux itself uses to setup it's own layouts.
Use `{} layout` to generate the layout string
for the current tmux configuration. This is
equivalent to running 

`
tmux list-windows -F \"#{{window_active}} #{{window_layout}}\" 
  | grep \"^1\" 
  | cut -d \" \" -f 2
`
 ",
        crate_name!()
    )
}

fn commands_long_help() -> String {
    format!(
        "This argument, like it's config file equivalent,
is a list of commands. These commands will 
be run in the panes of the tmux window that
will be opened by {:?}. The commands index 
(beginning with 0) corresponds to the pane
id. Pane id's can be found easily with 
`<prefix >q` in tmux.
 ",
        crate_name!()
    )
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

fn default_commands() -> Vec<String> {
    vec!["vim".to_string(), "ls".to_string()]
}

fn default_window_name() -> Option<String> {
    None
}

pub trait DmuxConf {
    fn from_homedir() -> Result<config::Config>;

    fn from_dir(dir: &PathBuf) -> Result<config::Config>;
}

impl DmuxConf for config::Config {

    fn from_homedir() -> Result<config::Config> {
        let mut settings = config::Config::default();
        let mut config_conf =
            dirs::config_dir().ok_or_else(|| anyhow!("Config dir couldn't be read"))?;
        config_conf.push("dmux/dmux.conf.xxxx");

        let mut home_conf =
            dirs::home_dir().ok_or_else(|| anyhow!("Home directory couldn't be found"))?;
        home_conf.push(".dmux.conf.xxxx");

        let mut mac_config =
            dirs::home_dir().ok_or_else(|| anyhow!("Home directory couldn't be found"))?;
        mac_config.push(".config/dmux/dmux.conf.xxx");
        Ok(settings
            // ~/dmux.conf.(yaml | json | toml)
            .merge(config::File::with_name(config_conf.to_str().unwrap()).required(false))?
            // ~/{xdg_config|.config}dmux.conf.(yaml | json | toml)
            .merge(config::File::with_name(home_conf.to_str().unwrap()).required(false))?
            .merge(config::File::with_name(mac_config.to_str().unwrap()).required(false))?
            // Add in settings from the environment (with a prefix of DMUX)
            // Eg.. `DMUX_SESSION_NAME=foo dmux` would set the `session_name` key
            .merge(config::Environment::with_prefix("DMUX"))?
            .to_owned())
    }

    fn from_dir(dir: &PathBuf) -> Result<config::Config> {
        let mut settings = config::Config::default();
        let mut config_conf = dir.clone();
        config_conf.push("dmux.conf.toml");
        Ok(settings
            // ~/dmux.conf.(yaml | json | toml)
            .merge(config::File::with_name(config_conf.to_str().unwrap()).required(true))?
            // Eg.. `DMUX_SESSION_NAME=foo dmux` would set the `session_name` key
            .merge(config::Environment::with_prefix("DMUX"))?
            .to_owned())
    }

}

fn config_file_settings(from_dir: Option<&str>) -> Result<config::Config> {
    // switch to confy perobably
    let default = WorkSpaceArgs::default();
    let conf = match from_dir {
        Some(dir) => {
            let path = PathBuf::from(dir);
            config::Config::from_dir(&path)
        },
        None => {
            config::Config::from_homedir()
        }
    };
    Ok(conf?
        .set_default("layout", default.layout)?
        // the trait `std::convert::From<i32>` is not implemented for `config::value::ValueKind`
        .set_default("number_of_panes", default.number_of_panes as i64)?
        .set_default("commands", default.commands)?
        .set_default("session_name", default.session_name)?
        .to_owned())
}

fn settings_config(settings: config::Config, target: Option<&str>) -> Result<WorkSpaceArgs> {
    if let Some(target) = target {
        let profile: WorkSpaceArgs = settings.get(target)?;
        return Ok(profile);
    }
    let profile: WorkSpaceArgs = settings.try_into()?;
    Ok(profile)
}

pub struct SelectArgs {
    pub workspace: WorkSpaceArgs,
}

pub enum CommandType {
    Open(OpenArgs),
    Select(SelectArgs),
    Pull(PullArgs),
    Layout,
}

// I don't like the repetition here
#[derive(Deserialize, Debug)]
pub struct WorkSpaceArgs {
    #[serde(default = "default_layout_checksum")]
    pub layout: String,
    #[serde(default = "default_session_name")]
    pub session_name: String,
    #[serde(default = "default_number_of_panes")]
    pub number_of_panes: i32,
    #[serde(default = "default_search_dir")]
    pub search_dir: PathBuf,
    #[serde(default = "default_commands")]
    pub commands: Vec<String>,
    #[serde(default = "default_window_name")]
    pub window_name: Option<String>,
}

impl Default for WorkSpaceArgs {
    fn default() -> Self {
        Self {
            window_name: None,
            layout: default_layout_checksum(),
            session_name: default_session_name(),
            number_of_panes: default_number_of_panes(),
            search_dir: dirs::home_dir().unwrap(),
            commands: default_commands(),
        }
    }
}

pub struct OpenArgs {
    pub workspace: WorkSpaceArgs,
    pub selected_dir: PathBuf,
}

#[derive(Debug)]
pub struct PullArgs {
    pub repo_url: String,
    pub target_dir: PathBuf,
    pub workspace: WorkSpaceArgs,
}

fn read_line_iter() -> Result<String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn select_dir(args: &clap::ArgMatches) -> Option<PathBuf> {
    if let Ok(selected_dir) = value_t!(args.value_of("selected_dir"), PathBuf) {
        Some(selected_dir)
    } else if grep_cli::is_readable_stdin() && !grep_cli::is_tty_stdin() {
        if let Ok(path) = read_line_iter() {
            Some(PathBuf::from(path))
        } else {
            None
        }
    } else {
        None
    }
}

fn build_workspace_args(args: &clap::ArgMatches) -> Result<WorkSpaceArgs> {
    let config_dir = if args.is_present("profile_from_dir") {
        args.value_of("selected_dir")
    } else {
        None
    };
    let settings = config_file_settings(config_dir)?;
    let conf_from_settings = settings_config(settings, args.value_of("profile"))?;
    let search_dir =
        value_t!(args.value_of("search_dir"), PathBuf).unwrap_or(conf_from_settings.search_dir);
    Ok(WorkSpaceArgs {
        window_name: value_t!(args.value_of("window_name"), String).ok(),
        session_name: value_t!(args.value_of("session_name"), String)
            .unwrap_or(conf_from_settings.session_name),
        layout: value_t!(args.value_of("layout"), String).unwrap_or(conf_from_settings.layout),
        number_of_panes: value_t!(args.value_of("number_of_panes"), i32)
            .unwrap_or(conf_from_settings.number_of_panes),
        commands: values_t!(args.values_of("commands"), String)
            .unwrap_or(conf_from_settings.commands),
        search_dir,
    })
}

fn expand_selected_dir(path: PathBuf) -> Result<PathBuf> {
    if path == PathBuf::from(".") {
        Ok(std::env::current_dir()?)
    } else {
        Ok(path)
    }
}

pub fn build_app() -> Result<CommandType> {
    let args = args();
    let workspace = build_workspace_args(&args)?;
    match args.subcommand_name() {
        None => {
            if let Some(selected_dir) = select_dir(&args) {
                Ok(CommandType::Open(OpenArgs {
                    workspace,
                    selected_dir: expand_selected_dir(selected_dir)?,
                }))
            } else {
                Ok(CommandType::Select(SelectArgs { workspace }))
            }
        }
        Some("clone") => {
            let clone_args = args
                .subcommand_matches("clone")
                .ok_or_else(|| anyhow!("Problem reading clones"))?;
            let repo_url = clone_args
                .value_of("repo")
                .ok_or_else(|| anyhow!("No repo specified, what should I clone?"))?
                .to_owned();
            Ok(CommandType::Pull(PullArgs {
                repo_url,
                target_dir: value_t!(args.value_of("target_dir"), PathBuf)
                    .unwrap_or_else(|_| dirs::home_dir().unwrap()),
                workspace,
            }))
        }

        Some("layout") => Ok(CommandType::Layout),
        Some(_) => Err(anyhow!("unexpected subcommand")),
    }
}
