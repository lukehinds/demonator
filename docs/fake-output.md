# Fake / Simulated Output

Fake output lets you pre-define what a command displays instead of running the
real thing. This is essential for:

- Demoing against systems that aren't available (APIs, cloud services, databases)
- Guaranteeing deterministic, clean output every time
- Showing idealized output without noise or irrelevant lines

## Basic usage

```yaml
steps:
  - text: "curl -s https://api.example.com/health"
    fake_output: |
      {"status": "healthy", "version": "2.1.0", "uptime": "14d 3h"}
    execute: false
```

The command is typed out as usual, but when executed the fake output is printed
instead of running the real command. Set `execute: false` to skip the real
command entirely.

## Typing out fake output

For dramatic effect, you can make the fake output appear character-by-character:

```yaml
steps:
  - text: "docker ps"
    fake_output: |
      CONTAINER ID   IMAGE            STATUS
      a1b2c3d4e5f6   myapp:latest     Up 3 hours
      f6e5d4c3b2a1   postgres:15      Up 3 hours
    output_speed: 60
    execute: false
```

The `output_speed` field controls how fast the output is typed, in characters
per second. If omitted, the output appears instantly.

## Running the real command too

If you omit `execute: false`, demonator will show the fake output AND run the
real command silently in the background. This is useful when the real command has
side effects you need (like creating files) but you want controlled output:

```yaml
steps:
  - text: "terraform init"
    fake_output: |
      Initializing provider plugins...
      - Finding latest version of hashicorp/aws...
      - Installing hashicorp/aws v5.31.0...

      Terraform has been successfully initialized!
    # execute defaults to true — terraform init actually runs, but output is faked
```

## Multi-line output

Use YAML block scalars for multi-line output:

```yaml
# Literal block (preserves newlines)
fake_output: |
  line 1
  line 2
  line 3

# Or a regular quoted string with \n
fake_output: "line 1\nline 2\nline 3\n"
```

## Combining with capture

Fake output and capture are independent — if you need to capture a value from
fake output, run the real command (with `execute: true`) and capture from that.
The fake output is just for display.
