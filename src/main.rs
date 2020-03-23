extern crate clap;
extern crate dirs;
extern crate interactor;
extern crate tmux_interface;
extern crate walkdir;

mod select;

use crate::tmux_interface::pane::PANE_ALL;
use clap::{App, Arg, SubCommand};
use interactor::*;
use select::Selector;
use std::env;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Output;
use std::process::{Command, Stdio};
use tmux_interface::session::SESSION_ALL;
use tmux_interface::window::WINDOW_ALL;
use tmux_interface::{
    AttachSession, DetachClient, Error, NewSession, NewWindow, SelectWindow, SendKeys, Sessions,
    SplitWindow, SwitchClient, TmuxInterface,
};

fn session_has_window(session_name: &str, window_name: &str) -> bool {
    let mut tmux = TmuxInterface::new();
    let mut contains = false;
    let windows = tmux
        .list_windows(Some(false), None, Some(session_name))
        .unwrap();
    for line in windows.lines() {
        println!("{:?}", line);
        if line.contains(window_name) {
            contains = true;
        }
    }
    println!("session found: {:?}", contains);
    return contains;
}

fn create_session(
    session_name: &str,
    window_name: &str,
    tmux: &mut TmuxInterface,
) -> Result<String, Error> {
    let new_session = NewSession {
        session_name: Some(session_name),
        window_name: Some(window_name),
        detached: Some(true),
        ..Default::default()
    };
    let ses = tmux.new_session(Some(&new_session));

    let attach = AttachSession {
        target_session: Some(session_name),
        ..Default::default()
    };
    tmux.attach_session(Some(&attach));
    return ses;
}

// impl<'a> Config<'a> {}

fn attach_or_create_window(
    session_name: &str,
    window_name: &str,
    dir: &str,
    tmux: &mut TmuxInterface,
) {
    if session_has_window(session_name, window_name) {
        println!("Session {} has window: {}", session_name, window_name);
        let select_window = SelectWindow {
            target_window: Some(window_name),
            last: Some(true),
            ..Default::default()
        };
        let result = tmux.select_window(Some(&select_window));
        println!("selection of window {:?}", result);
    } else {
        println!("session not found");
        let window = NewWindow {
            window_name: Some(window_name),
            target_window: Some(session_name),
            cwd: Some(dir),
            ..Default::default()
        };
        let result = tmux.new_window(Some(&window));
        println!("new window {:?}", result);
    }
}

fn switch_to_session(target_session: &str, tmux: &mut TmuxInterface) -> Result<Output, Error> {
    if in_tmux() {
        println!("In tmux, switching to the correct session");
        let switch = SwitchClient {
            target_session: Some(target_session),
            target_client: None,
            // this is to get it to use the current session, might not need
            ..Default::default()
        };
        return tmux.switch_client(Some(&switch));
    } else {
        println!("Outside of tmux, attaching to session");
        let attach = AttachSession {
            target_session: Some(target_session),
            // this is to get it to use the current session, might not need
            ..Default::default()
        };
        return tmux.attach_session(Some(&attach));
    }
}

// let split = SplitWindow {
//     detached: Some(false),
//     print: Some(true),
//     horizontal: Some(true),
//     ..Default::default()
// };
// let s = tmux.split_window(Some(&split));
// println!("split info {:?}", s);
fn split_window(session_name: &str, window_name: &str, tmux: &mut TmuxInterface) {
    // this is a fantastic universal way to manipulate an arbitrary tmux window/session
    // it also can probably be a more universal way to do a lot of the other stuff
    let target = format!("{}:{}.0", session_name, window_name);
    let split = SendKeys {
        target_pane: Some(target.as_str()),
        ..Default::default()
    };
    let keys = vec!["tmux split-window -hb", "Enter"];
    let s = tmux.send_keys(Some(&split), &keys);
    println!("split info {:?}", s);
}
// struct Config<'a> {
//     session_name: &'a str,
//     window_name: &'a str,
//     dir: &'a str,
//     tmux: &'a mut TmuxInterface<'a>,
// }
//
//
// we can configure the entire window with three simple bits of data, the number of windows, a relation of window to a command, and the layout String
// these should both be put into the eventual config struct

fn setup_layout(session_name: &str, window_name: &str, tmux: &mut TmuxInterface) {
    let target = format!("{}:{}.0", session_name, window_name);
    let split = SendKeys {
        target_pane: Some(target.as_str()),
        ..Default::default()
    };
    let keys = vec![
        "tmux select-layout 'f5fa,362x83,0,0{245x83,0,0,3,116x83,246,0,4}'",
        "Enter",
    ];
    let s = tmux.send_keys(Some(&split), &keys);
    println!("split info {:?}", s);

    //
}
fn main() {
    // session_has_window("zacharythomas", "violet");
    let mut tmux = TmuxInterface::new();

    // let dc = DetachClient {
    //     ..Default::default()
    // };
    // tmux.control_mode = Some(true);

    let session_name = "dev";
    let window_name = "baz";
    let dir = "/Users/zacharythomas/yakko_wakko";
    // let conf = Config {
    //     session_name,
    //     window_name,
    //     dir,
    //     tmux: &mut tmux,
    // };
    let check = Path::new(dir).is_dir();
    let create_ses = create_session(session_name, window_name, &mut tmux);
    let switch_to_session = switch_to_session(session_name, &mut tmux);
    attach_or_create_window(session_name, window_name, dir, &mut tmux);
    split_window(session_name, window_name, &mut tmux);
    setup_layout(session_name, window_name, &mut tmux);

    // let check = Path::new(dir).is_dir();
    // let create_ses = create_session(session_name, window_name, &mut tmux);
    // let switch_to_session = switch_to_session(session_name, &mut tmux);
    // attach_or_create_window(session_name, window_name, dir, &mut tmux);
    //
    //
    //
    //
    //
    //
    //

    // let sk = SendKeys::new();
    // let sent_keys = tmux.send_keys(Some(&sk), &vec!["echo", " foo", "<Cr>"]);
    // let win = attach_or_create_window(session_name, window_name, dir, &mut tmux);

    // if in_tmux() {
    //     // new window
    //     // setup layout
    // } else {
    //     // attach_session session_name
    // }
    // if !tmux.has_session(Some(session_name)).unwrap() {
    //     let new_session = NewSession {
    //         session_name: Some(session_name),
    //         attach: Some(true),
    //         window_name: Some("default"),
    //         parent_sighup: Some(true),
    //         ..Default::default()
    //     };
    //     let result = tmux.new_session(Some(&new_session));
    //     println!("connected to session result {:?}", result);
    //     switch_to_session(&session_name);
    // } else {
    //     let attach = AttachSession {
    //         target_session: Some(session_name),
    //         ..Default::default()
    //     };
    //     tmux.attach_session(Some(&attach));
    // }
    // println!("attached to session")
    // let find_win = FindWindow {
    //     ..Default::default()
    // };
    // let find = tmux.find_window(Some(&find_win), "default");
    // println!("result for find window{:?}", find);
}

fn args<'a>() -> clap::ArgMatches<'a> {
    return App::new("DMUX")
        .version("0.0.1")
        .author("Zdcthomas")
        .about("a nicer way to open up tmux workspaces")
        .arg(
            Arg::with_name("session")
                .short("s")
                .long("session_name")
                .help("specifies the session_name to attach to or create")
                .global(true)
                .empty_values(false),
        )
        .arg(
            Arg::with_name("dir")
                .short("d")
                .long("dir")
                .help("sets a parent dir to put newly cloned repos in")
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
        .subcommand(
            SubCommand::with_name("attach")
                .about("attaches to existing dev sessions")
                .arg(Arg::with_name("force").short("f").long("force")),
        )
        .get_matches();
}
fn in_tmux() -> bool {
    env::var("TMUX").is_ok()
}

// let layout = "e797,362x83,0,0{265x83,0,0,3,96x83,266,0,4}"

fn foo() {
    let matches = args();
    let session_name = matches
        .value_of("session")
        .unwrap_or(default_session_name());
    let search_dir = dirs::home_dir().unwrap();
    println!("arguments {:?}", matches);
    match matches.subcommand_name() {
        Some("attach") => AttachConfig::new(session_name).attach_to_sessions(),

        Some("clone") => clone_from(),
        None => {
            if let Some(selected_dir) = Selector::new(search_dir).select_dir() {
                let path = Path::new(selected_dir.as_str());
                let mut tmux = TmuxInterface::new();

                let sessions = Sessions::get(SESSION_ALL).unwrap();
                // create_session(session_name, &mut tmux);
                // open_dev_in(path);
            }
        }
        _ => unreachable!(),
    }
    // if let Some(repo) = matches.value_of("attach") {
    //
    //     println!("YOU'VE SELECTED ... {:?}", repo)
    // } else {
    //     println!("Don't attach to a seession")
    //     // println!("opening window")
    // }
    //
    //              Already running tmux               ||             no running tmux
    //                          |                      ||                   |
    //  desired Session exists || doesn't exist        ||       create session with window name
    //                  |            |
    //
    //
    //
    //
}

// fn create_session(session_name: &str, tmux: &mut TmuxInterface) {
//     let new_session = NewSession {
//         session_name: Some(session_name),
//         detached: Some(true),
//         window_name: Some("foo"),
//         parent_sighup: Some(true),
//         ..Default::default()
//     };
//     let result = tmux.new_session(Some(&new_session));
//     println!("{:?}", result);
// }

fn open_dev_in(path: &Path) {
    println!("the path is{:?}", path);
    println!("is dir? {:?}", path.is_dir());
}

struct AttachConfig {
    session_name: String,
}

impl AttachConfig {
    pub fn new<'a>(session_name: &'a str) -> AttachConfig {
        return AttachConfig {
            session_name: String::from(session_name),
        };
    }

    pub fn attach_to_sessions(&self) {
        if let Ok(_) = env::var("TMUX") {
            println!("Already in a tmux session dummy dum dum uwu ");
            return;
        }
        let mut tmux = TmuxInterface::new();
        let sessions = Sessions::get(SESSION_ALL).unwrap();
        println!("{:?}", sessions);
        let attach_session = AttachSession {
            target_session: Some(self.session_name.as_str()),
            ..Default::default()
        };
        if !tmux
            .attach_session(Some(&attach_session))
            .unwrap()
            .status
            .success()
        {
            println!("Session {} doesn't exist.", self.session_name)
        }
    }
}

fn default_session_name<'a>() -> &'a str {
    return "development";
}

fn clone_from() {
    println!("cloning the repo down")
}

// fn main() {
// let matches = App::new("DMUX")
//     .version("0.0.1")
//     .author("Zdcthom")
//     .about("a nicer way to open up tmux 'workspaces'")
//     .arg(
//         Arg::with_name("repo")
//             .short("r")
//             .long("repo")
//             .help("clones a repo from a git remote")
//             .takes_value(true),
//     )
//     .arg(
//         Arg::with_name("attach")
//         .short("a")
//         .long("attach")
//         .help("attaches to any running session")
//         )
//     .arg(
//         Arg::with_name("dir")
//             .short("d")
//             .long("dir")
//             .help("sets a parent dir to put newly cloned repos in")
//             .takes_value(true),
//     )
//     .get_matches();
// if let Some(repo) = matches.value_of("repo") {
//     // println!("YOU'VE SELECTED ... {:?}", repo)
// } else {
//     // println!("opening window")
// }
// select_dir();
// }
//

// println!("{:?}", sessions);
// let windows = Windows::get("zacharythomas", WINDOW_ALL).unwrap();
// // println!("{:?}", windows);
// let panes = Panes::get("zacharythomas:medit", PANE_ALL).unwrap();
// select_dir();
// // println!("{:?}", panes);
// // let mut tmux = TmuxInterface::new();
// // let new_session = NewSession {
// //     detached: Some(true),
// //     session_name: Some("dmux"),
// //     ..Default::default()
// // };
// // tmux.new_session(Some(&new_session)).unwrap();
// // let attach_session = AttachSession {
// //     target_session: Some("session_name"),
// //     ..Default::default()
// // };
