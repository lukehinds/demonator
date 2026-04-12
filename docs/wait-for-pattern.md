# Wait-for-Pattern

Wait-for-pattern lets you pause the demo until a specific regex pattern appears
in a command's output. Once matched, the command is terminated and the demo
advances. This is essential for demos involving servers, build tools, or any
process that takes variable time to produce a meaningful result.

## Basic usage

```yaml
steps:
  - text: "docker-compose up -d"

  - text: "docker-compose logs -f api"
    wait_for: "Listening on port 8080"
    timeout: 60

  - text: "curl localhost:8080/health"
```

Here, `docker-compose logs -f` runs indefinitely. Once the output contains
"Listening on port 8080", demonator kills the process and moves on.

## How it works

1. The command is spawned with piped stdout and stderr
2. Output is printed to the terminal in real time
3. Each line is checked against the `wait_for` regex
4. On match, the command is killed and the demo advances
5. If the command exits before the pattern matches, a warning is shown

## Options

| Field      | Default | Description                              |
|------------|---------|------------------------------------------|
| `wait_for` | --      | Regex pattern to match (uses Rust regex syntax) |
| `timeout`  | `30`    | Maximum seconds to wait before giving up |

## Regex patterns

The `wait_for` value is a full regex. Some examples:

```yaml
# Exact string match
wait_for: "Server started"

# Match a port number
wait_for: "Listening on port \\d+"

# Match any of several patterns
wait_for: "(ready|started|listening)"

# Case-insensitive (regex flag)
wait_for: "(?i)success"
```

## Use cases

### Waiting for a server to start

```yaml
- text: "python -m http.server 8000 &"
- text: "curl -s --retry 5 --retry-connrefused http://localhost:8000"
  wait_for: "<!DOCTYPE html>"
  timeout: 10
```

### Waiting for a build to finish

```yaml
- text: "cargo build --release 2>&1"
  wait_for: "Finished"
  timeout: 120
```

### Waiting for a container to be healthy

```yaml
- text: "docker logs -f mycontainer"
  wait_for: "ready to accept connections"
  timeout: 30
```

## Tips

- Always set a reasonable `timeout` — the default is 30 seconds
- The command is killed with SIGKILL when the pattern matches
- If the command exits on its own before the pattern matches, a warning is
  printed but the demo continues
- Output is printed to the terminal in real time as the command runs
