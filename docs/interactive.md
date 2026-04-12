# Expect-Style Interactive Commands

Some commands prompt for input during execution — confirmation dialogs, setup
wizards, interactive installers. The `interact` feature lets you pre-define
responses to these prompts so the demo flows smoothly.

## Basic usage

```yaml
steps:
  - text: "npm init"
    interact:
      - expect: "package name:"
        send: "my-cool-app"
      - expect: "version:"
        send: "1.0.0"
      - expect: "description:"
        send: "A demo project"
      - expect: "entry point:"
        send: "index.js"
      - expect: "Is this OK?"
        send: "yes"
```

## How it works

1. The command is spawned with piped stdin and stdout
2. Output is printed to the terminal as it arrives
3. Each `expect` pattern is checked against accumulated stdout
4. When a pattern matches, the `send` value (plus a newline) is written to stdin
5. The next `expect` pattern becomes active
6. After all interactions complete, remaining output is printed

## Interaction fields

| Field    | Description                                        |
|----------|----------------------------------------------------|
| `expect` | String to match in the command's stdout             |
| `send`   | Text to send to stdin when matched (newline added)  |

## Examples

### Confirming a destructive action

```yaml
- text: "rm -ri /tmp/old-data"
  interact:
    - expect: "remove"
      send: "y"
```

### Git interactive rebase (simplified)

```yaml
- text: "git commit --amend"
  interact:
    - expect: "# Please enter the commit message"
      send: ":wq"
```

### Multi-step installer

```yaml
- text: "curl -sSL https://install.example.com | sh"
  interact:
    - expect: "Install location"
      send: "/usr/local"
    - expect: "Add to PATH?"
      send: "y"
    - expect: "Enable telemetry?"
      send: "n"
```

## Limitations

- The `expect` match is a simple string contains check, not regex
- Interactions are processed in order — you cannot skip or reorder them
- Each interaction has a 30-second timeout; if the expected pattern doesn't
  appear, the interaction is abandoned
- Programs that disable echo or use raw terminal mode (like password prompts)
  may not work because output is piped rather than using a PTY
- stderr is inherited directly (not piped), so patterns must appear on stdout

## Tips

- Test your interact sequences with `--dry-run` first to see the flow
- Keep `send` values short and unambiguous
- If a program requires an empty response (just pressing Enter), use
  `send: ""`
- For commands that only need a `y/n` confirmation, this is much cleaner than
  piping `echo y |` into the command
