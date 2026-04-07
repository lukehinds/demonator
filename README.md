# demonator

Typewriter-style text display for terminal demos. Demonator reads a YAML config
file of commands and "types" them out character-by-character with realistic
timing, then executes each command when you press Enter.

## Install

```sh
cargo install --path .
```

## Usage

```sh
demonator              # uses demo.yml in the current directory
demonator -c my.yml    # use a custom config file
```

Press **Enter** after each step to run the command and advance to the next one.

## Configuration

Create a YAML file (default: `demo.yml`):

```yaml
speed: 20
delay: 50
jitter: 40
pause: 200
clear: true
prompt: "{green}~{reset} {blue}${reset} "

steps:
  - text: "git clone git@github.com:example/myproject.git"
  - text: "cargo build --release"
    speed: 28
  - text: "cargo test"
    speed: 28
  - text: "echo 'done!'"
    speed: 16
    jitter: 60
```

### Global options

| Option   | Default                              | Description                                      |
|----------|--------------------------------------|--------------------------------------------------|
| `prompt` | `{green}~{reset} {blue}${reset} `   | Prompt string displayed before each command       |
| `clear`  | `false`                              | Clear the terminal before starting                |
| `speed`  | —                                    | Characters per second (overrides `delay`)         |
| `delay`  | `50`                                 | Milliseconds between characters                   |
| `jitter` | `40`                                 | Percentage of random timing variation             |
| `pause`  | `200`                                | Extra milliseconds after punctuation              |

### Per-step overrides

Each step can override `speed`, `delay`, `jitter`, and `pause`. If both `speed`
and `delay` are set, `speed` takes priority.

### Prompt colors

Use color placeholders in the `prompt` string:

`{black}` `{red}` `{green}` `{yellow}` `{blue}` `{magenta}` `{cyan}` `{white}` `{bold}` `{dim}` `{reset}`

## License

MIT
