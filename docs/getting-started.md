# Getting Started

## Installation

```sh
cargo install --path .
```

Or build from source:

```sh
cargo build --release
cp target/release/demonator /usr/local/bin/
```

## Your first demo

Create a file called `demo.yml`:

```yaml
steps:
  - text: "echo 'Hello from demonator!'"
  - text: "date"
  - text: "uname -a"
```

Run it:

```sh
demonator
```

Each command is typed out character-by-character. Press **Enter** to execute it
and advance to the next step.

## Customizing the look

Add global options to control timing and appearance:

```yaml
speed: 25              # characters per second
jitter: 30             # random timing variation (%)
pause: 150             # extra delay after punctuation (ms)
clear: true            # clear terminal before starting
prompt: "{cyan}>{reset} "  # custom prompt

steps:
  - text: "echo 'looks different now'"
```

## Using a custom config file

```sh
demonator -c my-other-demo.yml
```

## What's next

- [Configuration reference](configuration.md) — all global and per-step options
- [Commentary and narration](commentary.md) — add explanatory text between commands
- [Fake output](fake-output.md) — simulate command output without running anything
- [Chapters](chapters.md) — organize demos into navigable sections
- [Auto-advance and recording](auto-advance.md) — record demos with asciinema
- [Interactive commands](interactive.md) — handle commands that prompt for input
- [Conditional steps](conditionals.md) — branch based on runtime state
- [Syntax highlighting](syntax-highlighting.md) — colorize commands as they are typed
- [Setup and teardown](setup-teardown.md) — hidden environment prep and cleanup
- [Wait-for-pattern](wait-for-pattern.md) — wait for output before advancing
- [Authoring workflow](authoring.md) — dry-run and hot reload for writing demos
