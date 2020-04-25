extern crate clap;
extern crate dirs;
extern crate interactor;
extern crate tmux_interface;
extern crate walkdir;

mod select;
mod tmux;

use clap::{App, Arg, SubCommand};
use select::Selector;
use std::collections::HashMap;
use std::env;
use std::io::BufRead;
use std::path::Path;
use std::process::Output;
use tmux::{Commands, Layout, WorkSpace};

use tmux_interface::session::SESSION_ALL;

fn in_tmux() -> bool {
    env::var("TMUX").is_ok()
}

fn default_session_name<'a>() -> &'a str {
    return "development";
}

fn clone_from() {
    println!("cloning the repo down")
}

fn path_to_window_name(path: &Path) -> &str {
    path.file_name()
        .expect("dir path contained invalid unicode")
        .to_str()
        .unwrap()
}

fn main() {
    let matches = App::new("DMUX")
        .version("0.0.1")
        .author("Zdcthom")
        .about("a nicer way to open up tmux 'workspaces'")
        .arg(
            Arg::with_name("repo")
                .short("r")
                .long("repo")
                .help("clones a repo from a git remote")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("attach")
                .short("a")
                .long("attach")
                .help("attaches to any running session"),
        )
        .arg(
            Arg::with_name("dir")
                .short("d")
                .long("dir")
                .help("sets a parent dir to put newly cloned repos in")
                .takes_value(true),
        )
        .get_matches();

    let session_name = matches
        .value_of("session")
        .unwrap_or(default_session_name());
    let search_dir = dirs::home_dir().unwrap();
    println!("arguments {:?}", matches);
    match matches.subcommand_name() {
        Some("attach") => println!("use tmux to attach to session/window"),

        Some("clone") => clone_from(),
        None => {
            if let Some(selected_dir) = Selector::new(search_dir).select_dir() {
                let path = Path::new(selected_dir.as_str());
                println!("do the tmux sent to {:?}", path);
                let layout = Layout {
                    window_count: 2,
                    layout_string: String::from("34ed,230x56,0,0{132x56,0,0,3,97x56,133,0,222}"),
                };
                let mut commands = HashMap::new();
                commands.insert(0, String::from("nvim"));
                commands.insert(1, String::from("pipes.sh"));
                let workspaces = WorkSpace {
                    session_name: default_session_name(),
                    window_name: path_to_window_name(path),
                    dir: path.to_str().expect("oops on path str"),
                    layout,
                    commands,
                };
                tmux::setup_workspace(workspaces);
                // create_session(session_name, &mut tmux);
                // open_dev_in(path);
            }
        }
        _ => unreachable!(),
    }

    // let check = Path::new(dir).is_dir();
    // let create_ses = create_session(session_name, window_name, &mut tmux);
    // let switch_to_session = switch_to_session(session_name, &mut tmux);
    // attach_or_create_window(session_name, window_name, dir, &mut tmux);
    //
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

// let layout = "e797,362x83,0,0{265x83,0,0,3,96x83,266,0,4}"

fn foo() {
    // let matches = args();
    // let session_name = matches
    //     .value_of("session")
    //     .unwrap_or(default_session_name());
    // let search_dir = dirs::home_dir().unwrap();
    // println!("arguments {:?}", matches);
    // match matches.subcommand_name() {
    //     Some("attach") => AttachConfig::new(session_name).attach_to_sessions(),

    //     Some("clone") => clone_from(),
    //     None => {
    //         if let Some(selected_dir) = Selector::new(search_dir).select_dir() {
    //             let path = Path::new(selected_dir.as_str());
    //             let mut tmux = TmuxInterface::new();

    //             let sessions = Sessions::get(SESSION_ALL).unwrap();
    //             // create_session(session_name, &mut tmux);
    //             // open_dev_in(path);
    //         }
    //     }
    //     _ => unreachable!(),
    // }
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

// fn main() {
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
