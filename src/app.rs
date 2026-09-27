use anyhow::Result;
use clap::{crate_authors, crate_description, crate_name, crate_version, Arg};

use std::fs::canonicalize;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
// const DEFAULT_LAYOUT: &str = "34ed,230x56,0,0{132x56,0,0,3,97x56,133,0,222}";

fn fzf_available() -> bool {
    Command::new("fzf")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

// `long_help` borrows for the lifetime of the `Command`, and these strings are
// built at run time, so they have to outlive the builder.
fn leak(string: String) -> &'static str {
    Box::leak(string.into_boxed_str())
}

fn command(fzf_available: bool) -> clap::Command<'static> {
    clap::Command::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::new("selected_dir")
                .help("Open this directory directly without starting a selector")
                .takes_value(true)
                // if fzf isn't available, this needs to be specified
                .required(!fzf_available),
        )
        .arg(
            Arg::new("session_name")
                .short('s')
                .long("session_name")
                .help("specify a specific session name to run")
                .takes_value(true),
        )
        .arg(
            Arg::new("window_name")
                .short('w')
                .long("window")
                .help("specify the window name")
                .takes_value(true),
        )
        .arg(
            Arg::new("number_of_panes")
                .short('p')
                .long("panes")
                .help("the number of panes to generate.")
                .takes_value(true),
        )
        .arg(
            // One value per `-c`, so that the trailing path stays a path.
            Arg::new("commands")
                .short('c')
                .long("commands")
                .help("commands to run in panes")
                .long_help(leak(commands_long_help()))
                .takes_value(true)
                .multiple_occurrences(true)
                .use_value_delimiter(true),
        )
        .arg(
            // We should use validator here
            Arg::new("layout")
                .short('l')
                .long("layout")
                .help("specify the window layout (layouts are dependent on the number of panes)")
                .long_help(leak(layout_long_help()))
                .takes_value(true),
        )
        .arg(
            Arg::new("profile")
                .short('P')
                .long("profile")
                .help("Use a different configuration profile.")
                .takes_value(true),
        )
        .arg(
            Arg::new("search_dir")
                .short('d')
                .long("dir")
                .help("override of the dir to select from.")
                .takes_value(true),
        )
        .subcommand(
            clap::Command::new("clone")
                .about("clones a git repository, and then opens a workspace in the repo")
                .arg(
                    Arg::new("repo")
                        .help("specifies the repo to clone from")
                        .required(true),
                )
                .arg(
                    Arg::new("name")
                        .short('n')
                        .long("name")
                        .help("sets the local name for the cloned repo")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("target_dir")
                        .short('t')
                        .long("target")
                        .help("the dir to clone into. Defaults to your home dir.")
                        .takes_value(true),
                ),
        )
        .subcommand(
            clap::Command::new("layout").about("generates the current layout string from tmux"),
        )
}

fn args() -> clap::ArgMatches {
    command(fzf_available()).get_matches()
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

/// Replaces a leading `~` with the home dir. A shell does this for an
/// argument, but nothing does it for a value that came from a config file.
fn expand_tilde(path: PathBuf, home: Option<&Path>) -> PathBuf {
    let home = match home {
        Some(home) => home,
        None => return path,
    };
    let mut components = path.components();
    match components.next() {
        Some(std::path::Component::Normal(first)) if first == "~" => {
            home.join(components.as_path())
        }
        _ => path,
    }
}

fn home_or_fallback(home: Option<PathBuf>) -> PathBuf {
    home.unwrap_or_else(|| PathBuf::from("."))
}

// `config::File::with_name` swaps the final extension for each format it
// supports, so every candidate needs a placeholder extension. Without it
// `dmux.conf` would resolve to `dmux.toml`.
const CONFIG_NAME: &str = "dmux.conf.xxxx";

/// Where dmux looks for a config file, lowest precedence first. A later file
/// overrides an earlier one.
fn config_paths(config_dir: Option<&Path>, home_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home_dir) = home_dir {
        paths.push(home_dir.join(format!(".{}", CONFIG_NAME)));
    }
    if let Some(config_dir) = config_dir {
        paths.push(config_dir.join("dmux").join(CONFIG_NAME));
    }
    if let Some(home_dir) = home_dir {
        paths.push(home_dir.join(".config").join("dmux").join(CONFIG_NAME));
    }

    // On Linux `dirs::config_dir()` is `~/.config`, so two of these name one
    // file. Merging it twice is wasted work and confuses the precedence.
    let mut seen = std::collections::HashSet::new();
    paths.retain(|path| seen.insert(path.clone()));
    paths
}

fn default_search_dir() -> PathBuf {
    home_or_fallback(dirs::home_dir())
}

fn default_layout_checksum() -> String {
    "34ed,230x56,0,0{132x56,0,0,3,97x56,133,0,222}".to_string()
}

fn default_session_name() -> String {
    "dev".to_string()
}

fn default_number_of_panes() -> u8 {
    2
}

fn default_commands() -> Vec<String> {
    vec!["vim".to_string(), "ls".to_string()]
}

fn default_window_name() -> Option<String> {
    None
}

fn config_file_settings() -> Result<config::Config> {
    // switch to confy perobably
    let default = WorkSpaceArgs::default();
    let mut settings = config::Config::default();

    for path in config_paths(dirs::config_dir().as_deref(), dirs::home_dir().as_deref()) {
        let path = path
            .to_str()
            .ok_or_else(|| anyhow!("{:?} is not valid UTF-8", path))?
            .to_owned();
        settings.merge(config::File::with_name(&path).required(false))?;
    }

    Ok(settings
        // Add in settings from the environment (with a prefix of DMUX)
        // Eg.. `DMUX_SESSION_NAME=foo dmux` would set the `session_name` key
        .merge(config::Environment::with_prefix("DMUX"))?
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
    // Open a given selected dir passed in either through stdin or args
    Open(OpenArgs),
    // Select workspace dir from a fuzzy finder
    Select(SelectArgs),
    // Pull a repo from a git repository and then open that dir
    Pull(PullArgs),
    // Generate a tmux layout for the setup of panes in the current window
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
    pub number_of_panes: u8,
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
            search_dir: default_search_dir(),
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
    pub name: Option<String>,
    pub workspace: WorkSpaceArgs,
}

fn read_line_iter() -> Result<String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn select_dir(args: &clap::ArgMatches) -> Option<PathBuf> {
    if let Ok(selected_dir) = args.value_of_t::<PathBuf>("selected_dir") {
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

/// An argument wins over the config file, and the config file wins over the
/// defaults.
fn merge_workspace_args(
    args: &clap::ArgMatches,
    conf_from_settings: WorkSpaceArgs,
    home: Option<&Path>,
) -> WorkSpaceArgs {
    WorkSpaceArgs {
        window_name: args
            .value_of_t::<String>("window_name")
            .ok()
            .or(conf_from_settings.window_name),
        session_name: args
            .value_of_t::<String>("session_name")
            .unwrap_or(conf_from_settings.session_name),
        layout: args
            .value_of_t::<String>("layout")
            .unwrap_or(conf_from_settings.layout),
        number_of_panes: args
            .value_of_t::<u8>("number_of_panes")
            .unwrap_or(conf_from_settings.number_of_panes),
        commands: args
            .values_of_t::<String>("commands")
            .unwrap_or(conf_from_settings.commands),
        search_dir: expand_tilde(
            args.value_of_t::<PathBuf>("search_dir")
                .unwrap_or(conf_from_settings.search_dir),
            home,
        ),
    }
}

fn build_workspace_args(args: &clap::ArgMatches) -> Result<WorkSpaceArgs> {
    let settings = config_file_settings()?;
    let conf_from_settings = settings_config(settings, args.value_of("profile"))?;
    Ok(merge_workspace_args(
        args,
        conf_from_settings,
        dirs::home_dir().as_deref(),
    ))
}

pub fn build_app() -> Result<CommandType> {
    let args = args();
    let workspace = build_workspace_args(&args)?;
    match args.subcommand_name() {
        None => {
            if let Some(selected_dir) = select_dir(&args) {
                Ok(CommandType::Open(OpenArgs {
                    workspace,
                    selected_dir: canonicalize(selected_dir)?,
                }))
            } else {
                Ok(CommandType::Select(SelectArgs { workspace }))
            }
        }
        Some("clone") => {
            let clone = args
                .subcommand_matches("clone")
                .ok_or_else(|| anyhow!("Problem reading clones"))?;
            let repo_url = clone
                .value_of("repo")
                .ok_or_else(|| anyhow!("No repo specified, what should I clone?"))?
                .to_owned();
            Ok(CommandType::Pull(PullArgs {
                repo_url,
                target_dir: expand_tilde(
                    clone
                        .value_of_t::<PathBuf>("target_dir")
                        .unwrap_or_else(|_| home_or_fallback(dirs::home_dir())),
                    dirs::home_dir().as_deref(),
                ),
                name: clone.value_of("name").map(str::to_owned),
                workspace,
            }))
        }

        Some("layout") => Ok(CommandType::Layout),
        Some(_) => Err(anyhow!("unexpected subcommand")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches_from(argv: &[&str]) -> clap::ArgMatches {
        command(true).try_get_matches_from(argv).unwrap()
    }

    fn home() -> PathBuf {
        PathBuf::from("/home/zach")
    }

    #[test]
    fn commands_given_with_a_delimiter_leave_the_selected_dir_alone() {
        let matches = matches_from(&["dmux", "-c", "nvim,fish", "/tmp/project"]);
        assert_eq!(
            matches.values_of_t::<String>("commands").unwrap(),
            vec!["nvim".to_owned(), "fish".to_owned()]
        );
        assert_eq!(matches.value_of("selected_dir"), Some("/tmp/project"));
    }

    #[test]
    fn commands_given_as_repeated_flags_leave_the_selected_dir_alone() {
        let matches = matches_from(&["dmux", "-c", "nvim", "-c", "fish", "/tmp/project"]);
        assert_eq!(
            matches.values_of_t::<String>("commands").unwrap(),
            vec!["nvim".to_owned(), "fish".to_owned()]
        );
        assert_eq!(matches.value_of("selected_dir"), Some("/tmp/project"));
    }

    #[test]
    fn a_command_keeps_its_spaces() {
        let matches = matches_from(&["dmux", "-c", "npm i", "/tmp/project"]);
        assert_eq!(
            matches.values_of_t::<String>("commands").unwrap(),
            vec!["npm i".to_owned()]
        );
    }

    #[test]
    fn clone_takes_a_target_dir() {
        let matches = matches_from(&["dmux", "clone", "https://x/y.git", "-t", "/tmp/repos"]);
        let clone = matches.subcommand_matches("clone").unwrap();
        assert_eq!(clone.value_of("target_dir"), Some("/tmp/repos"));
    }

    #[test]
    fn clone_takes_a_local_name() {
        let matches = matches_from(&["dmux", "clone", "https://x/y.git", "-n", "renamed"]);
        let clone = matches.subcommand_matches("clone").unwrap();
        assert_eq!(clone.value_of("name"), Some("renamed"));
    }

    #[test]
    fn a_config_window_name_survives_when_no_flag_is_given() {
        let conf = WorkSpaceArgs {
            window_name: Some("from-config".to_owned()),
            ..Default::default()
        };
        let merged = merge_workspace_args(
            &matches_from(&["dmux", "/tmp/project"]),
            conf,
            Some(&home()),
        );
        assert_eq!(merged.window_name, Some("from-config".to_owned()));
    }

    #[test]
    fn a_window_name_flag_beats_the_config() {
        let conf = WorkSpaceArgs {
            window_name: Some("from-config".to_owned()),
            ..Default::default()
        };
        let merged = merge_workspace_args(
            &matches_from(&["dmux", "-w", "from-flag", "/tmp/project"]),
            conf,
            Some(&home()),
        );
        assert_eq!(merged.window_name, Some("from-flag".to_owned()));
    }

    #[test]
    fn a_config_search_dir_gets_its_tilde_expanded() {
        let conf = WorkSpaceArgs {
            search_dir: PathBuf::from("~/dev"),
            ..Default::default()
        };
        let merged = merge_workspace_args(
            &matches_from(&["dmux", "/tmp/project"]),
            conf,
            Some(&home()),
        );
        assert_eq!(merged.search_dir, PathBuf::from("/home/zach/dev"));
    }

    #[test]
    fn expand_tilde_replaces_a_leading_tilde() {
        let home = PathBuf::from("/home/zach");
        assert_eq!(
            expand_tilde(PathBuf::from("~/dev"), Some(&home)),
            PathBuf::from("/home/zach/dev")
        );
    }

    #[test]
    fn expand_tilde_expands_a_bare_tilde() {
        let home = PathBuf::from("/home/zach");
        assert_eq!(
            expand_tilde(PathBuf::from("~"), Some(&home)),
            PathBuf::from("/home/zach")
        );
    }

    #[test]
    fn expand_tilde_leaves_an_absolute_path_alone() {
        let home = PathBuf::from("/home/zach");
        assert_eq!(
            expand_tilde(PathBuf::from("/var/tmp"), Some(&home)),
            PathBuf::from("/var/tmp")
        );
    }

    #[test]
    fn expand_tilde_only_treats_a_leading_tilde_as_home() {
        let home = PathBuf::from("/home/zach");
        assert_eq!(
            expand_tilde(PathBuf::from("/var/~/tmp"), Some(&home)),
            PathBuf::from("/var/~/tmp")
        );
    }

    #[test]
    fn expand_tilde_leaves_the_path_alone_without_a_home_dir() {
        assert_eq!(
            expand_tilde(PathBuf::from("~/dev"), None),
            PathBuf::from("~/dev")
        );
    }

    #[test]
    fn home_or_fallback_does_not_panic_without_a_home_dir() {
        assert_eq!(home_or_fallback(None), PathBuf::from("."));
    }

    #[test]
    fn config_paths_do_not_repeat_a_file_when_xdg_config_is_dot_config() {
        // On Linux `dirs::config_dir()` IS `~/.config`, so two of the three
        // candidates name the same file.
        let paths = config_paths(
            Some(Path::new("/home/zach/.config")),
            Some(Path::new("/home/zach")),
        );
        assert_eq!(paths.len(), 2, "repeated config paths: {:?}", paths);
    }

    #[test]
    fn every_config_path_uses_the_same_extension_placeholder() {
        // `config` swaps the final extension for each format it supports, so
        // the placeholder is what makes `dmux.conf.toml` resolve. Two different
        // placeholders hide the fact that two paths name one file.
        for path in config_paths(
            Some(Path::new("/home/zach/.config")),
            Some(Path::new("/home/zach")),
        ) {
            assert_eq!(
                path.extension().and_then(|e| e.to_str()),
                Some("xxxx"),
                "{:?}",
                path
            );
        }
    }

    #[test]
    fn config_paths_cover_the_documented_locations() {
        let paths = config_paths(
            Some(Path::new("/home/zach/Library/Application Support")),
            Some(Path::new("/home/zach")),
        );
        let shown: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
        assert!(shown.iter().any(|p| p.contains("/home/zach/.dmux.conf")), "{:?}", shown);
        assert!(
            shown
                .iter()
                .any(|p| p.contains("/home/zach/.config/dmux/dmux.conf")),
            "{:?}",
            shown
        );
        assert!(
            shown
                .iter()
                .any(|p| p.contains("Library/Application Support/dmux/dmux.conf")),
            "{:?}",
            shown
        );
    }
}
