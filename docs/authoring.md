# Authoring Workflow

Demonator includes two features to streamline the process of writing and testing
demo configs: **dry-run mode** and **hot reload**.

## Dry-run mode

Preview the entire demo flow without typing animation or command execution:

```sh
demonator --dry-run
demonator --dry-run -c my-demo.yml
```

Dry-run output shows:

- Each step with the configured prompt
- Chapter boundaries (if using chapters)
- Setup and teardown commands
- Annotations for special features:
  - `[no-exec]` — step won't execute the command
  - `[fake-output]` — step has pre-defined output (shown indented with `>`)
  - `[wait-for]` — step waits for a pattern
  - `[interactive]` — step has expect/send interactions
  - `[conditional]` — step has `if` or `unless` conditions
  - `[capture]` — step captures output

Example dry-run output:

```
[setup]
  mkdir -p /tmp/demo
  echo 'ready'

── Setup ──
~ $ git clone git@github.com:example/myproject.git
~ $ cd myproject

── Build ──
This builds in release mode.
~ $ cargo build --release  [wait-for]
~ $ cargo test

── Deploy ──
~ $ kubectl apply -f deploy.yml  [conditional]

[teardown]
  rm -rf /tmp/demo
```

## Hot reload

Watch the config file and automatically re-run the demo when it changes:

```sh
demonator --watch
demonator --watch -c my-demo.yml
```

The workflow:

1. The demo runs normally (you press Enter through steps)
2. After the last step, demonator prints "watching for changes..."
3. Edit and save your YAML file
4. The screen clears and the demo restarts automatically
5. Press **Ctrl+C** to exit

This gives you a tight feedback loop when authoring demos — make a change, see
the result immediately.

## Combining both

A good authoring workflow:

1. Start with `--dry-run` to sketch out the structure:
   ```sh
   demonator --dry-run -c my-demo.yml
   ```

2. Switch to `--watch` to iterate on timing and output:
   ```sh
   demonator --watch -c my-demo.yml
   ```

3. Do a final run without flags to rehearse the real presentation:
   ```sh
   demonator -c my-demo.yml
   ```

## Tips

- Dry-run is instant — use it to quickly check that your YAML parses correctly
  and the step order makes sense
- Hot reload polls the file every 500ms, so changes are picked up almost
  immediately
- Hot reload clears the screen before each re-run, so you always see a fresh
  start
- Setup commands still run in dry-run mode (so your environment is ready), but
  teardown does not
