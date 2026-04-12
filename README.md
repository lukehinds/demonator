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
demonator --dry-run    # preview demo flow without executing
demonator --watch      # re-run demo when config file changes
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

| Option         | Default                              | Description                                      |
|----------------|--------------------------------------|--------------------------------------------------|
| `prompt`       | `{green}~{reset} {blue}${reset} `   | Prompt string displayed before each command       |
| `clear`        | `false`                              | Clear the terminal before starting                |
| `speed`        | --                                   | Characters per second (overrides `delay`)         |
| `delay`        | `50`                                 | Milliseconds between characters                   |
| `jitter`       | `40`                                 | Percentage of random timing variation             |
| `pause`        | `200`                                | Extra milliseconds after punctuation              |
| `highlight`    | `false`                              | Syntax-highlight commands as they are typed       |
| `auto_advance` | --                                   | Milliseconds to wait before auto-advancing (no Enter needed) |

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

## Features

### Commentary / narration blocks

Print styled explanatory text between commands. No prompt is shown and no
command is executed.

```yaml
steps:
  - comment: "First, let's clone the repository."
    style: dim
  - text: "git clone git@github.com:example/myproject.git"
  - comment: "Now build it."
    style: bold
  - text: "cargo build --release"
```

Supported styles: `dim`, `bold`, `italic`, `red`, `green`, `yellow`, `blue`,
`magenta`, `cyan`. Defaults to `dim` when omitted.

### Fake / simulated output

Show pre-defined output instead of (or in addition to) running the real command.
Great for demos against systems that aren't available.

```yaml
steps:
  - text: "curl -s https://api.example.com/health"
    fake_output: |
      {"status": "healthy", "version": "2.1.0"}
    execute: false

  - text: "docker ps"
    fake_output: |
      CONTAINER ID   IMAGE          STATUS
      a1b2c3d4e5f6   myapp:latest   Up 3 hours
    output_speed: 40     # type out the fake output character-by-character
    execute: false
```

| Field          | Description                                                  |
|----------------|--------------------------------------------------------------|
| `fake_output`  | Text to display as command output                            |
| `output_speed` | Characters per second for typing fake output (instant if omitted) |
| `execute`      | Set to `false` to skip running the real command (default `true`) |

### Auto-advance mode

Skip the Enter-key wait and advance automatically. Useful for recording demos
with tools like asciinema.

```yaml
auto_advance: 1500   # global: wait 1.5s between steps

steps:
  - text: "echo fast"
    wait: 500        # per-step override
  - text: "echo slow"
    wait: 3000
```

Record with asciinema:

```sh
asciinema rec -c "demonator -c auto-demo.yml"
```

### Wait-for-pattern

Wait until a regex pattern appears in command output before advancing. The
command is killed once the pattern matches (useful for long-running processes
like log tailing).

```yaml
steps:
  - text: "docker-compose up -d"
  - text: "docker-compose logs -f api"
    wait_for: "Listening on port 8080"
    timeout: 60
  - text: "curl localhost:8080/health"
```

| Field      | Default | Description                           |
|------------|---------|---------------------------------------|
| `wait_for` | --      | Regex pattern to match in output      |
| `timeout`  | `30`    | Seconds to wait before giving up      |

### Setup and teardown

Run hidden commands before and after the demo. The audience never sees these.

```yaml
setup:
  - "docker-compose down -v 2>/dev/null"
  - "rm -rf /tmp/demo-workspace"
  - "mkdir -p /tmp/demo-workspace"

teardown:
  - "docker-compose down -v"
  - "rm -rf /tmp/demo-workspace"

steps:
  - text: "ls /tmp/demo-workspace"
```

### Chapters

Organize steps into named sections. A chapter header is displayed when entering
each chapter.

```yaml
chapters:
  - name: "Setup"
    steps:
      - text: "git clone repo"
      - text: "cd myproject"
  - name: "Build"
    steps:
      - text: "cargo build --release"
  - name: "Test"
    steps:
      - text: "cargo test"
```

When using chapters, navigation is available at each Enter prompt:

| Input  | Action                          |
|--------|---------------------------------|
| Enter  | Advance to next step            |
| `n`    | Skip to next chapter            |
| `p`    | Go back to previous chapter     |
| `1`-`9`| Jump to chapter by number       |

### Expect-style interaction

Send keystrokes to interactive commands when expected output patterns appear.

```yaml
steps:
  - text: "npm init"
    interact:
      - expect: "package name:"
        send: "my-cool-app"
      - expect: "version:"
        send: "1.0.0"
      - expect: "Is this OK?"
        send: "yes"
```

Each `expect` pattern is matched against accumulated stdout. When matched, the
`send` value (plus a newline) is written to the command's stdin.

### Syntax highlighting

Colorize commands as they are typed. Enable globally:

```yaml
highlight: true

steps:
  - text: "cat README.md | grep -i 'install' > output.txt"
  - text: "echo $HOME"
  - text: "docker build --tag myapp:latest ."
```

Highlighting rules:

| Element                    | Color        |
|----------------------------|--------------|
| Command (first word)       | Bold white   |
| Flags (`--foo`, `-v`)      | Yellow       |
| Strings (`"..."`, `'...'`) | Green        |
| Pipes, redirects (`\|`, `>`) | Cyan       |
| Variables (`$FOO`)         | Magenta      |
| Comments (`# ...`)         | Green        |

### Conditional steps

Run steps only when a captured variable exists (and is non-empty) or doesn't
exist.

```yaml
steps:
  - text: "which docker"
    capture:
      name: has_docker
      pattern: "(.*)"

  - text: "docker build -t myapp ."
    if: has_docker

  - text: "podman build -t myapp ."
    unless: has_docker
```

### Dry-run mode

Preview the entire demo flow without typing animation or command execution:

```sh
demonator --dry-run
```

This shows every step, chapter boundaries, annotations for special features
(fake output, conditionals, interactive, etc.), and setup/teardown commands.

### Hot reload

Watch the config file for changes and automatically re-run the demo:

```sh
demonator --watch
```

The screen is cleared and the demo restarts whenever the YAML file is modified.
Press **Ctrl+C** to exit.

## License

MIT
