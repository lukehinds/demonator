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

### Capturing command output

Steps can capture values from command output using regex and inject them into
later steps. Add a `capture` block with a `name` and a `pattern` containing one
capture group:

```yaml
steps:
  - text: "nono run --detached --allow-cwd --profile claude-code -- claude"
    capture:
      name: session_id
      pattern: "Started detached session (\\w+)"

  - text: "nono attach {session_id}"
```

The first capture group `(\\w+)` is extracted from stdout and stored under the
given name. Subsequent steps can reference it with `{session_id}`. Multiple
captures can be used across different steps and combined in a single command.

### Prompt colors

Use color placeholders in the `prompt` string:

`{black}` `{red}` `{green}` `{yellow}` `{blue}` `{magenta}` `{cyan}` `{white}` `{bold}` `{dim}` `{reset}`

## License

MIT
