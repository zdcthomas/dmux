extern crate tmux_interface;
use std::collections::HashMap;
use std::process::Output;
use std::result::Result;
use tmux_interface::pane::PANE_ALL;
use tmux_interface::session::SESSION_ALL;
use tmux_interface::window::WINDOW_ALL;
use tmux_interface::{
    NewSession, NewWindow, SelectWindow, SendKeys, Sessions, SplitWindow, SwitchClient,
    TmuxInterface, Windows,
};

#[allow(dead_code)]
pub struct Tmux {
    sessions: Vec<Session>,
}

pub struct WorkSpace<'a> {
    pub session_name: &'a str,
    pub window_name: &'a str,
    pub dir: &'a str,
    pub layout: Layout,
    pub commands: Commands,
}

pub fn default_layout_checksum<'a>() -> &'a str {
    "34ed,230x56,0,0{132x56,0,0,3,97x56,133,0,222}"
}

#[allow(dead_code)]
pub fn setup_workspace(workspace: WorkSpace) -> Tmux {
    let mut tmux = Tmux::new();
    println!("finding or creating window");
    let to_be_deleted: Option<String>;
    let session: &mut Session;

    if let Some(sess) = tmux.find_session(workspace.session_name) {
        to_be_deleted = None;
        session = sess;
    } else {
        session = tmux
            .create_session(workspace.session_name)
            .expect("could not create session");

        let deletion = session.windows.first().unwrap().name.clone();
        to_be_deleted = Some(deletion);
    }
    session
        .setup_workspace(workspace)
        .attach()
        .expect("couldn't attach to window");
    if let Some(delete_name) = to_be_deleted {
        session
            .remove_window(delete_name.as_str())
            .expect("Could not remove temp window");
    }

    //     if let Some(window) = session.find_window("foo") {
    //       window.attach().expect("couldn't attach to window");
    //     } else {
    //       let window = session
    //         .create_window("foo", "/Users/zacharythomas/dev/Toskr/")
    //         .expect("could not create window");
    //       println!("setting up layout");
    //       window.setup_layout(layout).unwrap();
    //       println!("sending initial commands");
    //       window.initial_command(commands);
    //       window.attach().expect("couldn't attach to window");
    //     }
    tmux
}

#[allow(dead_code)]
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

pub struct Session {
    windows: Vec<Window>,
    name: String,
}

impl Session {
    pub fn remove_window(&mut self, window_name: &str) -> Result<Output, tmux_interface::Error> {
        TmuxInterface::new().kill_window(Some(false), Some(self.target(window_name, 0).as_str()))
    }

    fn target(&self, window_name: &str, pane: i32) -> String {
        format!("{}:{}.{}", self.name, window_name, pane)
    }

    #[allow(dead_code)]
    pub fn setup_workspace(&mut self, workspace: WorkSpace) -> &mut Window {
        if self.has_window(workspace.window_name) {
            return self
                .find_window(workspace.window_name)
                .expect("window destroyed during operation");
        }
        let window = self
            .create_window(workspace.window_name, workspace.dir)
            .expect("could not create window");
        println!("setting up layout");
        window.setup_layout(workspace.layout).unwrap();
        println!("sending initial commands");
        window.initial_command(workspace.commands);
        return window;
    }

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

pub struct Layout {
    // I wouldn't need two things here if I could just parse the tmux layout checksum
    pub window_count: i32,
    pub layout_checksum: String,
}

pub type Commands = HashMap<i32, String>;

struct Window {
    panes: Vec<Pane>,
    session_name: String,
    number_of_panes: i32,
    name: String,
}

impl Window {
    pub fn default_commands() -> Commands {
        let mut commands = HashMap::new();
        commands.insert(0, String::from("nvim"));
        commands.insert(1, String::from("echo yo"));
        commands
    }

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
            layout.layout_checksum
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
        let select = SwitchClient {
            target_session: Some(target.as_str()),
            ..Default::default()
        };
        let mut tmux = TmuxInterface::new();
        tmux.switch_client(Some(&select))
    }

    #[allow(dead_code)]
    pub fn initial_command(&mut self, commands: Commands) {
        for (pane, command) in commands {
            if let Some(pane) = self.get_pane(pane) {
                pane.send_keys(vec![command.as_str(), "Enter"])
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
        let layout = Layout {
            window_count: 2,
            layout_checksum: String::from("34ed,230x56,0,0{132x56,0,0,3,97x56,133,0,222}"),
        };

        let mut commands = HashMap::new();
        commands.insert(0, String::from("nvim"));
        commands.insert(1, String::from("echo yo"));

        setup_workspace(WorkSpace {
            session_name: "dev",
            window_name: "toskr",
            dir: "/Users/zacharythomas/dev/Toskr/",
            layout,
            commands,
        });
    }
}
