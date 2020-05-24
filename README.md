# DMUX
### (*D*evelopment t*MUX*)
##### new names definitely being considered

## What is this?
If you use tmux a lot, then you probably have a script that looks like this:
```bash
tmux new-window -n $WINDOW_NAME
tmux split-window -h
tmux select-pane -t 0
tmux send-keys "fish" C-m
tmux send-keys "nvim" C-m
tmux select-pane -t 1
tmux send-keys "fish" C-m
tmux resize-pane -R 80
tmux -2 attach-session -t $SESSION
```
Scripts like the one above set up and open a tmux session with specified commands and layout.
But if I wanted to have another pane that ran my tests, or another for note taking, I'd have to create an entirely new script.
I also wanted to be able to use some program like [fzf](https://github.com/junegunn/fzf) or [skim](https://github.com/lotabout/skim) to pick a directory to open.
This got super annoying.

Dmux aims to handle all of this for you.
Its main job is to open up configurable "workspaces" in whatever directory you want.
It also allows you to specify everything you would normally set in a script like the one above.

For example, the above script using dmux would be:
`dmux -c nvim fish <path>`
Then if I wanted the workspace to open 3 panes instead of two, I could add:
`dmux -c nvim fish "npm i" -p 3 <path>`

But say I wanted to use [fzf](https://github.com/junegunn/fzf) to select a dir to open up. 
Well, if I have it installed on my system, then I just have to leave off the <path> argument and dmux will automatically open an [fzf](https://github.com/junegunn/fzf) selector, populated with directories to choose from.

If this part is a bit slow to get started, no worries, you can speed up the dir searching by installing [fd](https://github.com/sharkdp/fd).

You can also use whatever combination of dir searching, selector, or hardcoded path you want by piping a path into dmux:
`fd -td | fzf | dmux`
or having a path argument:
`dmux <path>`

## Why another Tmux Manager?
There's a ton of other fantastic projects out there that also do similar things that you should check out:
* [tmuxinator](https://github.com/tmuxinator/tmuxinator)
* [tmuxomatic](https://github.com/oxidane/tmuxomatic) - Unmaintained
* [teamocil](https://github.com/remi/teamocil) - Unmaintained

#### So Why did I put together Dmux? 
* Dmux is a single binary that doesn't depend on a language to run.
* Other tools (like potentially the most popular manager [tmuxinator](https://github.com/tmuxinator/tmuxinator)) are based around a system of "projects" which have a specific root directory. This makes it difficult to reuse these configurations. Dmux on the on the other hand is based around directory agnostic profiles that can be run on any root directory.
* Because of dmux being agnostic of root dir, it also focuses on quickly selecting and opening directories. You can easily set up selection scripts to pipe a dir into dmux, or if you have fzf installed dmux will use that when run without arguments to let you select a dir.
* These 'profiles' also mix very well with command line arguments (all workspace settings can be set on either) and can therefore be easily extended in scripts or bindings.

## Installation 

##### macOS
``` bash
brew tap zdcthomas/tools
brew install dmux
```

Or if you have rust installed
``` bash
cargo install dmux
```
 
##### AUR
```
Coming soon
```

## Usage
* `dmux` alone will use `fzf` to open up a list of dirs in `~`. This is equivalent to saying `fd -td . ~/ | fzf | dmux`
* `dmux <path>` or `<path> | dmux` will open the workspace in the provided path
* `dmux clone` will clone a git repo and open the repo in a workspace
* `dmux layout` will describe the current Tmux layout (this is mostly )
* `dmux --help` for more information


## Configuration
Dmux's configuration tries to be very inclusive in terms of config file types. Dmux supports 
`JSON, YAML, TOML,` and ` HJSON`. It also supports a variety of paths including
`~/.dmux.conf.{file_type}`
`~/.config/dmux/dmux.conf.{file_type}`
and on Linux 
`$XDG_CONFIG_HOME/dmux/dmux.conf.{file_type}`

#### Example Configuration File
  This config file has a profile named `javascript` and defaults set
##### TOML
```toml
layout = "5e09,281x67,0,0{133x67,0,0,17,147x67,134,0[147x33,134,0,18,147x33,134,34{73x33,134,34,136,73x33,208,34[73x16,208,34,164,73x16,208,51,165]}]}"
session_name = "development"
number_of_panes = 5
commands = ["nvim", "fish"]

[javascript]
number_of_panes = 3
session_name = "frontend"
commands = ["nvim", "fish", "yarn watch"]
```

## External deps
Currently dmux relies on [fzf](https://github.com/junegunn/fzf) to select a target dir to open the workspace in.
If you have [fd](https://github.com/sharkdp/fd) installed dmux will use it to speed up dir searching.

## Potential features
- [X] Config file to be read on startup
- [X] Args for layout string
- [X] Profiles in config that represent sets of configuration.
- [X] Config/Arg for dir search command
- [X] Optionally uses fd for a faster/async dir search
- [X] Subcommand to describe current layout
- [ ] Subcommand for killing windows from fzf
- [ ] Subcommand for generating default configuration file
- [ ] Config/Arg for dir search depth
- [ ] One-off commands that once completed, kill the pane they're in, E.G `npm i` or `mix deps.get`
- [ ] dmux.local.{yml|json|toml} file so that specific dirs can have specific layouts. This is dangerous because dmux allows config to run arbitrary commands, which could be used to be malicious
- [ ] Switch to skim to avoid external deps


## Bugs
#### please submit bugs as issues and I'll add them here
