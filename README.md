# wlsplit

Basic speedrun timer for Wayland compositors using wlr-layer-shell (wlroots/kwin)

[![Search](screenshots/screenshot.png?raw=true)](screenshots/screenshot.png?raw=true)
# Usage

For the simplest case, simply execute `wlsplit <filename>` and a split file will be generated and immediately used.

Some optional flags can be passed to change the content of that generated file:

- `--game`: Game name to use
- `--category`: Run category (e.g. "any%")
- `--splits`: A comma separated list of splits to use (e.g. "Tutorial,Boss 1,Firelink Shrine" etc)

See `wlsplit --help` for more.

wlsplit does not support any direct commands, instead it is meant to be controlled via socket, for which `wlsplitctl` can be used.  
Available commands are:

- start
- split
- skip
- pause
- reset
- quit

I would recommend binding these commands as hotkeys in your compositor so that they can be used while a game is in focus.

# Installation

## Requirements

 - `freetype-devel`
 - `fontconfig-devel`

For installation into `~/.cargo/bin` simply clone the repo and run: `cargo install --path .`

# Configuration

A configuration file with the defaults is automatically created in `.config/wlsplit/wlsplit.toml`.
Current configuration support is still rather rudimentary and will hopefully be improved.