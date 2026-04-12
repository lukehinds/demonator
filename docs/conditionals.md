# Conditional Steps

Conditional steps let you branch the demo based on runtime state. Steps can be
gated on whether a captured variable exists or not.

## Basic usage

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

If `which docker` succeeds, it outputs a path like `/usr/bin/docker` which gets
captured into `has_docker`. The `docker build` step runs because the variable is
set. The `podman build` step is skipped.

If `which docker` fails (docker not installed), there's no output to capture, so
`has_docker` is never set. The `docker build` step is skipped and `podman build`
runs instead.

## How conditions are evaluated

| Field    | Step runs when...                                         |
|----------|-----------------------------------------------------------|
| `if`     | The named variable exists AND is non-empty (after trim)   |
| `unless` | The named variable does NOT exist OR is empty             |

Both `if` and `unless` can be used on the same step — both conditions must be
satisfied for the step to run.

## Examples

### Feature detection

```yaml
steps:
  - text: "python3 --version 2>/dev/null"
    capture:
      name: python_version
      pattern: "Python (.*)"

  - comment: "Python detected, installing dependencies..."
    if: python_version
  - text: "pip3 install -r requirements.txt"
    if: python_version

  - comment: "Python not found, skipping Python setup."
    unless: python_version
```

Note: `if`/`unless` also works on comment steps when deserialized as command
steps. For comment-only conditionals, use a command step with `execute: false`:

```yaml
  - text: "echo 'Python not found, skipping.'"
    unless: python_version
    execute: false
    fake_output: ""
```

### Platform-specific commands

```yaml
steps:
  - text: "uname -s"
    capture:
      name: os_name
      pattern: "(Linux|Darwin)"

  - text: "brew install jq"
    if: os_name   # only if uname produced output (macOS or Linux)
```

### Multi-step conditional flow

```yaml
steps:
  - text: "test -f .env"
    capture:
      name: has_env
      pattern: "(.*)"

  - text: "cp .env.example .env"
    unless: has_env

  - text: "source .env && echo $DATABASE_URL"
    if: has_env
```

## Tips

- Variable names match the `name` field from a `capture` block
- A variable containing only whitespace is treated as empty (not set)
- Conditions are evaluated at runtime, so the same demo can behave differently
  on different machines
- Use `--dry-run` to see which steps have conditions — they're annotated with
  `[conditional]`
- Captured variables persist for the entire demo, so a capture in chapter 1 can
  be used in a condition in chapter 3
