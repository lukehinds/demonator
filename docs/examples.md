# Examples

Ready-to-use demo configs. Copy and customize for your own presentations.

## Basic demo

A simple demo with a few commands:

```yaml
speed: 20
clear: true

steps:
  - text: "echo 'Hello from demonator!'"
  - text: "date"
  - text: "uname -a"
```

## Full-featured demo

Showcases chapters, commentary, conditionals, fake output, and syntax
highlighting:

```yaml
speed: 22
jitter: 35
pause: 150
clear: true
highlight: true

setup:
  - "mkdir -p /tmp/demonator-demo"

teardown:
  - "rm -rf /tmp/demonator-demo"

chapters:
  - name: "Introduction"
    steps:
      - comment: "Welcome to the demonator full-feature demo!"
        style: bold
      - comment: "This demo showcases commentary, highlighting, chapters, and more."
      - pause

  - name: "Basic Commands"
    steps:
      - comment: "Let's start with some basic commands."
        style: cyan
      - text: "echo 'Hello from demonator!'"
      - text: "date '+%Y-%m-%d %H:%M:%S'"
      - text: "ls -la /tmp/demonator-demo"

  - name: "Fake Output"
    steps:
      - comment: "Sometimes you need to fake command output for a demo."
      - text: "curl -s https://api.example.com/v2/status"
        fake_output: |
          {
            "status": "operational",
            "version": "2.4.1",
            "uptime": "45d 12h 33m",
            "services": {
              "api": "healthy",
              "database": "healthy",
              "cache": "healthy"
            }
          }
        output_speed: 50
        execute: false

  - name: "Conditionals"
    steps:
      - comment: "Conditional steps adapt the demo to the environment."
      - text: "which git"
        capture:
          name: has_git
          pattern: "(.*)"
      - text: "git --version"
        if: has_git
      - text: "echo 'git is not installed'"
        unless: has_git

  - name: "Wrap Up"
    steps:
      - comment: "That's the tour! Navigate chapters with n/p or jump with 1-5."
        style: green
      - text: "echo 'Demo complete!'"
```

## API deployment demo

Fake a full deployment workflow without any real infrastructure:

```yaml
speed: 24
clear: true
highlight: true

steps:
  - comment: "Let's check our API health and deploy a new version."
    style: bold

  - text: "curl -s https://api.myapp.com/health | jq ."
    fake_output: |
      {
        "status": "healthy",
        "version": "1.9.2",
        "database": "connected",
        "cache_hit_rate": "94.2%"
      }
    execute: false

  - comment: "Looks good. Now let's deploy v2.0.0."

  - text: "kubectl set image deploy/api api=myapp:2.0.0"
    fake_output: "deployment.apps/api image updated"
    execute: false

  - text: "kubectl rollout status deploy/api"
    fake_output: |
      Waiting for deployment "api" rollout to finish: 1 of 3 updated replicas are available...
      Waiting for deployment "api" rollout to finish: 2 of 3 updated replicas are available...
      deployment "api" successfully rolled out
    output_speed: 30
    execute: false

  - text: "curl -s https://api.myapp.com/health | jq .version"
    fake_output: '"2.0.0"'
    execute: false

  - comment: "Deployment complete!"
    style: green
```

## Recording-ready demo

Auto-advance config designed for asciinema recording:

```yaml
speed: 28
auto_advance: 1500
clear: true
highlight: true

steps:
  - comment: "Setting up the project..."
    style: cyan

  - text: "mkdir -p /tmp/my-project && cd /tmp/my-project"
    wait: 1000

  - text: "echo 'fn main() { println!(\"Hello, world!\"); }' > main.rs"
    wait: 1000

  - text: "cat main.rs"
    wait: 2000

  - comment: "All done! Clean and simple."
    style: green
```

Record it:

```sh
asciinema rec -c "demonator -c examples/recording.yml" demo.cast
```

## Capture and variable substitution

Chain commands using captured output:

```yaml
speed: 20
clear: true

steps:
  - text: "nono run --detached --allow-cwd --profile claude-code -- claude"
    capture:
      name: session_id
      pattern: "Started detached session (\\w+)"

  - text: "nono attach {session_id}"
```

## Interactive command

Automate responses to an interactive prompt:

```yaml
steps:
  - text: "npm init"
    interact:
      - expect: "package name:"
        send: "my-app"
      - expect: "version:"
        send: "1.0.0"
      - expect: "description:"
        send: "A demo project"
      - expect: "Is this OK?"
        send: "yes"
```
