# DMUX
## (Development tMUX)
### new names definitely being considered

## Installation 
macOS 
```
brew tap zdcthomas/tools
brew install dmux
```

## Usage
* `dmux <path>` or `<path> | dmux` will open the workspace in the provided path
* `dmux` alone will use `fzf` to open up a list of dirs in `~`. This is equivalent to saying `fd -td . ~/ | fzf | dmux`
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
- [ ] Subcommand for killing windows from fzf
- [ ] Subcommand to describe current layout
- [ ] Config/Arg for dir search depth
- [ ] One off commands that once completed, kill the pane their in, E.G `npm i` `mix deps.get`
- [ ] dmux.local.xxxx file so that specific dirs can have specific layouts. This is dangerous because dmux allows config to run arbitrary commands, which could be used to be malicious
- [ ] Switch to skim to avoid external deps


## Bugs
#### please submit bugs as issues and I'll add them here
