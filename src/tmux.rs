extern crate tmux_interface;
use std::collections::HashMap;
use std::process::Output;
use std::result::Result;
use tmux_interface::pane::PANE_ALL;
use tmux_interface::session::SESSION_ALL;
use tmux_interface::window::WINDOW_ALL;
use tmux_interface::{
    NewSession, NewWindow, SelectWindow, SendKeys, Sessions, SplitWindow, TmuxInterface, Windows,
};

#[allow(dead_code)]
struct Tmux {
    sessions: Vec<Session>,
}

impl Tmux {
    #[allow(dead_code)]
    pub fn new() -> Tmux {
        Tmux {
            sessions: Session::all_sessions(),
        }
    }

    #[allow(dead_code)]
    pub fn send_keys(
        session_name: &str,
        window_name: &str,
        pane: i32,
        keys: Vec<&str>,
    ) -> Result<Output, tmux_interface::Error> {
        let target = format!("{}:{}.{}", session_name, window_name, pane);
        let split = SendKeys {
            target_pane: Some(target.as_str()),
            ..Default::default()
        };
        TmuxInterface::new().send_keys(Some(&split), &keys)
    }

    #[allow(dead_code)]
    fn find_session(&mut self, name: &str) -> Option<&mut Session> {
        for sess in self.sessions.iter_mut() {
            if sess.name == name {
                return Some(sess);
            }
        }
        None
    }

    #[allow(dead_code)]
    fn has_session(&self, name: &str) -> bool {
        self.sessions.iter().any(|s| s.name == name)
    }

    #[allow(dead_code)]
    fn create_session(&mut self, name: &str) -> Option<&mut Session> {
        let mut tmux = TmuxInterface::new();
        let new_session = NewSession {
            detached: Some(true),
            session_name: Some(name),
            ..Default::default()
        };
        tmux.new_session(Some(&new_session))
            .expect("Could not create new session");
        // let sess = tmux_interface::Session::from_str(name, SESSION_ALL).unwrap();
        // self.sessions.push(Session::from_interface(sess));
        // self.sessions.last_mut()
        self.sessions = Session::all_sessions();
        self.find_session(name)
    }

    #[allow(dead_code)]
    pub fn find_or_create_session(&mut self, name: &str) -> Option<&mut Session> {
        if self.has_session(name) {
            self.find_session(name)
        } else {
            self.create_session(name)
        }
    }
}

struct Session {
    windows: Vec<Window>,
    name: String,
}

impl Session {
    #[allow(dead_code)]
    pub fn all_sessions() -> Vec<Session> {
        let sessions = Sessions::get(SESSION_ALL).unwrap();
        Session::from_interface_list(sessions)
    }
    #[allow(dead_code)]
    pub fn from_interface_list(sessions: tmux_interface::Sessions) -> Vec<Session> {
        sessions
            .into_iter()
            .map(|s| Session::from_interface(s))
            .collect()
    }
    #[allow(dead_code)]
    pub fn from_interface(session: tmux_interface::Session) -> Session {
        let name = session.name.clone().unwrap();
        Session {
            windows: Window::all_in_session(name.as_str()),
            name,
        }
    }

    #[allow(dead_code)]
    fn find_window(&mut self, name: &str) -> Option<&mut Window> {
        for win in self.windows.iter_mut() {
            if win.name == name {
                return Some(win);
            }
        }
        None
    }

    #[allow(dead_code)]
    fn has_window(&self, name: &str) -> bool {
        self.windows.iter().any(|w| w.name == name)
    }

    #[allow(dead_code)]
    pub fn find_or_create_window(&mut self, name: &str, dir: &str) -> Option<&mut Window> {
        if self.has_window(name) {
            self.find_window(name)
        } else {
            self.create_window(name, dir)
        }
    }

    #[allow(dead_code)]
    fn create_window(&mut self, name: &str, dir: &str) -> Option<&mut Window> {
        let window = NewWindow {
            window_name: Some(name),
            target_window: Some(self.name.as_str()),
            cwd: Some(dir),
            detached: Some(true),
            ..Default::default()
        };
        // Yuck, I really hate this but tmux interface returns a string from new_window
        TmuxInterface::new()
            .new_window(Some(&window))
            .expect("Could not create new window");
        self.windows = Window::all_in_session(self.name.as_str());
        self.find_window(name)
    }
}

struct Layout {
    window_count: i32,
    layout_string: String,
}
type Commands<'a> = HashMap<i32, &'a str>;

struct Window {
    panes: Vec<Pane>,
    session_name: String,
    number_of_panes: i32,
    name: String,
}

impl Window {
    #[allow(dead_code)]
    fn from_interface(win: tmux_interface::Window, session_name: String) -> Window {
        let name = session_name.clone();
        let panes =
            tmux_interface::Panes::get(win.name.clone().unwrap().as_str(), PANE_ALL).unwrap();
        Window {
            panes: Pane::from_interface_list(
                panes,
                name.as_str(),
                win.name.clone().unwrap().as_str(),
            ),
            session_name,
            number_of_panes: win.panes.unwrap() as i32,
            name: win.name.unwrap(),
        }
    }

    #[allow(dead_code)]
    pub fn all_in_session(session_name: &str) -> Vec<Window> {
        let windows = Windows::get(session_name, WINDOW_ALL).unwrap();
        windows
            .into_iter()
            .map(|w| Window::from_interface(w, String::from(session_name)))
            .collect()
    }

    #[allow(dead_code)]
    pub fn send_keys(&self, keys: Vec<&str>) -> Result<Output, tmux_interface::Error> {
        Tmux::send_keys(self.session_name.as_str(), self.name.as_str(), 0, keys)
        // let target = format!("{}:{}.0", self.session_name, self.name);
        // let split = SendKeys {
        //     target_pane: Some(target.as_str()),
        //     ..Default::default()
        // };
        // TmuxInterface::new().send_keys(Some(&split), &keys)
    }

    fn target(&self, pane: i32) -> String {
        format!("{}:{}.{}", self.session_name, self.name, pane)
    }

    #[allow(dead_code)]
    fn split_window(&mut self) -> Result<String, tmux_interface::Error> {
        let target = self.target(0);
        let split = SplitWindow {
            target_pane: Some(target.as_str()),
            ..Default::default()
        };
        let mut tmux = TmuxInterface::new();
        let split_result = tmux.split_window(Some(&split));
        self.reload_panes();
        split_result
        // let target = format!("tmux split-window -t {}", self.target(0));
        // self.send_keys(vec![target.as_str(), "Enter"]).unwrap();
    }

    #[allow(dead_code)]
    pub fn setup_layout(&mut self, layout: Layout) -> Result<Output, tmux_interface::Error> {
        self.reload_panes();
        println!(" pre number_of_panes {}", self.number_of_panes);
        println!("layout window_count {}", layout.window_count);
        if self.number_of_panes < layout.window_count {
            for _x in self.number_of_panes..layout.window_count {
                self.split_window().expect("couldn't split window");
            }
        }
        // if self.number_of_panes > layout.window_count {
        //     for _x in layout.window_count..self.number_of_panes {
        //         let target = format!("tmux kill-pane -t {}", self.target(0));
        //         self.send_keys(vec![target.as_str(), "Enter"]).unwrap();
        //     }
        // }
        let tmux_command = format!(
            "tmux select-layout -t {} \"{}\"",
            self.target(0),
            layout.layout_string
        );
        self.reload_panes();
        self.send_keys(vec![tmux_command.as_str(), "Enter"])
    }

    #[allow(dead_code)]
    fn get_pane(&mut self, pane: i32) -> Option<&Pane> {
        self.panes.iter().find(|p| p.index == pane)
    }

    #[allow(dead_code)]
    pub fn attach(&self) -> Result<Output, tmux_interface::Error> {
        let target = self.target(0);
        println!("attaching to {}", target);
        let select = SelectWindow {
            target_window: Some(target.as_str()),
            ..Default::default()
        };
        let mut tmux = TmuxInterface::new();
        tmux.select_window(Some(&select))
    }

    #[allow(dead_code)]
    pub fn initial_command(&mut self, commands: Commands) {
        for (pane, command) in commands {
            if let Some(pane) = self.get_pane(pane) {
                pane.send_keys(vec![command, "Enter"])
                    .expect("could not send command");
            } else {
                println!("pane {} not found", pane);
                println!(
                    "available panes in window {:?} are: {:?}",
                    self.name, self.panes
                );
            }
        }
    }

    #[allow(dead_code)]
    fn reload_panes(&mut self) {
        let target = format!("{}:{}.0", self.session_name, self.name);
        let panes = tmux_interface::Panes::get(target.as_str(), PANE_ALL).unwrap();
        self.panes =
            Pane::from_interface_list(panes, self.session_name.as_str(), self.name.as_str());
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct Pane {
    session_name: String,
    window_name: String,
    index: i32,
}

impl Pane {
    #[allow(dead_code)]
    pub fn send_keys(&self, keys: Vec<&str>) -> Result<Output, tmux_interface::Error> {
        Tmux::send_keys(
            self.session_name.as_str(),
            self.window_name.as_str(),
            self.index,
            keys,
        )
    }

    #[allow(dead_code)]
    pub fn from_interface_list(
        panes: tmux_interface::Panes,
        session_name: &str,
        window_name: &str,
    ) -> Vec<Pane> {
        panes
            .into_iter()
            .map(|p| Pane::from_interface(p, session_name, window_name))
            .collect()
    }

    #[allow(dead_code)]
    pub fn from_interface(
        interface: tmux_interface::Pane,
        session_name: &str,
        window_name: &str,
    ) -> Pane {
        Pane {
            index: interface.index.unwrap() as i32,
            session_name: session_name.to_string(),
            window_name: window_name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let mut tmux = Tmux::new();
        let session = tmux.find_or_create_session("dev").unwrap();
        let layout = Layout {
            window_count: 2,
            layout_string: String::from("34ed,230x56,0,0{132x56,0,0,3,97x56,133,0,222}"),
        };
        let mut commands = HashMap::new();
        commands.insert(0, "nvim");
        commands.insert(1, "echo yo");
        println!("finding or creating window");
        let mut window = session
            .find_or_create_window("foo", "/Users/zacharythomas/dev/Toskr/")
            .unwrap();
        println!("setting up layout");
        window.setup_layout(layout).unwrap();
        println!("sending initial commands");
        window.initial_command(commands);
        window.attach().expect("couldn't attach to window");

        // .send_keys(vec!["echo hello", "Enter"]);
        // let after = tmux.sessions.first().unwrap().windows.len();
        // assert_eq!(after, initial);
    }

    // let dir = ask_for_dir()
    // tmux::new().find_or_create_session("dev")
    // start dmux
    // get input from either skim or fzf can't decide which -> Directory name or path
    // create session (either "dev" or something from args) ->
    //      in that session create a window (name: end of the path, working_dir: path) -> window :good
    //          on that window make pane arrangement
}
