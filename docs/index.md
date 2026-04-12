# demonator

**Typewriter-style text display for terminal demos.**

Demonator reads a YAML config file and "types" commands out character-by-character
with realistic timing, then executes each command when you press Enter. It makes
live terminal demos look natural and polished.

<div class="grid cards" markdown>

- :material-rocket-launch: **[Getting Started](getting-started.md)**

    Install demonator and create your first demo in under a minute.

- :material-cog: **[Configuration](configuration.md)**

    Full reference for all global options, per-step overrides, and CLI flags.

- :material-palette-outline: **[Features](#features)**

    Commentary, fake output, chapters, highlighting, conditionals, and more.

- :material-file-document-multiple: **[Examples](examples.md)**

    Ready-to-use demo configs for common scenarios.

</div>

## Quick start

```sh
cargo install --path .
```

Create `demo.yml`:

```yaml
speed: 20
clear: true
highlight: true

steps:
  - comment: "Let's build the project."
    style: cyan
  - text: "cargo build --release"
  - text: "cargo test"
  - text: "echo 'All done!'"
```

Run it:

```sh
demonator
```

## Features

| Feature | Description |
|---------|-------------|
| [Commentary](commentary.md) | Styled narration text between commands |
| [Fake output](fake-output.md) | Pre-defined command output for offline demos |
| [Syntax highlighting](syntax-highlighting.md) | Colorize commands as they are typed |
| [Chapters](chapters.md) | Named sections with keyboard navigation |
| [Auto-advance](auto-advance.md) | Hands-free mode for recording with asciinema |
| [Wait-for-pattern](wait-for-pattern.md) | Pause until regex matches in output |
| [Setup & teardown](setup-teardown.md) | Hidden environment prep and cleanup |
| [Interactive commands](interactive.md) | Expect-style responses to prompts |
| [Conditional steps](conditionals.md) | Branch based on captured variables |
| [Authoring tools](authoring.md) | Dry-run preview and hot reload |

## How it works

1. Demonator reads your YAML config
2. For each step, it types the command character-by-character with realistic jitter
3. You press **Enter** to execute the command
4. The real command runs (or fake output is shown)
5. Repeat until the demo is done

The result looks like a human typing commands live — but with perfect accuracy
and timing every time.
