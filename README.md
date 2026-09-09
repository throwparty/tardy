# Tardy

Never be late to meetings again. Bar application for showing your current/next
meeting, quickly joining it on click, and showing future meetings on hover.

______________________________________________________________________

## Usage

`tardy`, like many GNOME apps, sources calendar and event information from
GNOME Online Accounts (GOA) and the Evolution Data Server (EDS). You'll need
these two services to be configured, your online account to be added, and to
check that GNOME Calendar or another EDS consumer can see your calendar events.

| Command | Description |
| --- | --- |
| `tardy daemon --format=FORMAT` | Poll for live meetings and pop the fullscreen overlay when one starts, emitting current and future meeting information for your bar app |
| `tardy list` | List meetings from the calendars available on this machine |
| `tardy open` | Open the currently shown meeting in your browser |
| `tardy test takeover` | Show the fullscreen meeting-start overlay with a demo meeting |
| `tardy test notify` | Send the desktop notification reminder with a demo meeting |

### `tardy daemon`

Polls every 30 seconds (override with `--interval`) for live meetings and
pops a fullscreen takeover overlay when one starts. Deduplicated per recurring
UID, so a meeting that recurs daily only fires its overlay once.

```text
tardy daemon [OPTIONS]

Options:
      --interval <INTERVAL>  Seconds between polls (default 30)
      --format <FORMAT>      Emit status line on stdout in FORMAT [possible values: none, ashell, waybar]
  -h, --help                 Print help
```

With `--format=ashell|waybar` it additionally prints a
[waybar](https://github.com/Alexays/Waybar)-compatible JSON status line on
every poll, showing the next meeting in `text` (plus `class`, `tooltip`, and a
date-grouped tooltip; `alt` is emitted for ashell). Wire it into a waybar/ashell
custom module.

All-day events (holidays, out-of-office, birthdays) are listed in the tooltip
but never occupy the one-line text, never mark the module live, and never pop
the takeover — otherwise a single bank holiday would own your bar all day.

#### ashell

```toml
# ashell (~/.config/ashell/config.toml): fullscreen overlay for live meetings
[[CustomModule]]
name = "Tardy"
type = "Button"
command = "tardy open"
listen_cmd = "tardy daemon --format=ashell"

[modules]
right = ["Tardy"]
```

[ashell doesn't show tooltips](https://github.com/MalpenZibo/ashell/pull/914),
but I'm sure that'll change fairly soon.

#### Waybar

Module keys are `custom/<name>`, listed in your bar like any other module.
We enter the tardy binary in `exec` and let it run as a continuous script:

```jsonc
// waybar (~/.config/waybar/config.jsonc)
"custom/tardy": {
    "format": "{text}",
    "exec": "tardy daemon --format=waybar",
    "on-click": "tardy open",
    "exec-on-event": false,
    "return-type": "json",
    "tooltip": true
}

// and in your bar:
"modules-right": ["custom/tardy"]
```

`return-type` must be `json`: tardy emits the `text`/`class`/`tooltip` fields
waybar expects (`alt` is an ashell-ism, ignored by waybar). <code>#custom-tardy.<class></code>
lets you style `style.css` by state:

```css
#custom-tardy.live {
    color: #9ece6a;
}
```

Because tardy loops itself and never exits, don't set `interval`; use
`exec-on-event: false` so a click runs `tardy open` without killing/restarting
the module, and give the full path to the binary if it isn't on `PATH`
(e.g. `result/bin/tardy` from a nix build).

### `tardy list`

Prints upcoming meetings (default five days ahead; override with `--days`) as
pretty JSON.

```text
tardy list [OPTIONS]

Options:
      --days <DAYS>  How many days ahead to look (default 5)
```

### `tardy open`

Opens the currently shown meeting — i.e. the same meeting the status line
displays — in your browser via `xdg-open`. The link is taken from the event's
conference URL (`X-GOOGLE-CONFERENCE`, the `URL` property, or a Meet link
embedded in the description), falling back to the location.

______________________________________________________________________

## Installation

### Cargo

```shell
cargo install --path crates/tardy
```

Alternatively build the release binary in-place:

```shell
cargo build --release && install -m755 target/release/tardy ~/.local/bin/tardy
```

### Nix

The repository bundles a flake (`nix/flake.nix`) that builds `tardy` with the
rust-overlay stable toolchain and wraps it with the Wayland/XKB libraries it
dlopens at runtime:

```shell
nix build ./nix#default
# result/bin/tardy
```

`--format ashell` requires libwayland/libxkbcommon on `LD_LIBRARY_PATH` at
runtime; the wrapped `nix build` result sets that for you.

### Dev shell

```shell
nix develop ./nix
```

or, with direnv, `use flake ./nix` in `.envrc` (see `sample.envrc`). The shell
provides the Rust toolchain, `cargo` tooling (auditable/binstall/deny),
`pkg-config`, and the Wayland libraries. `just build`, `just test`,
`just lint`, and `just fmt` drive cargo, clippy, and treefmt.
