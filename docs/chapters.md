# Chapters

Chapters let you organize a demo into named sections. Each chapter gets a
visible header when it starts, and the presenter can navigate between chapters
during the demo.

## Defining chapters

Use `chapters` instead of `steps` at the top level:

```yaml
chapters:
  - name: "Setup"
    steps:
      - text: "git clone git@github.com:example/myproject.git"
      - text: "cd myproject"

  - name: "Build"
    steps:
      - text: "cargo build --release"
      - comment: "Build complete."

  - name: "Test"
    steps:
      - text: "cargo test"

  - name: "Deploy"
    steps:
      - text: "kubectl apply -f deploy.yml"
      - text: "kubectl rollout status deploy/myapp"
```

You cannot use both `steps` and `chapters` in the same config file.

## Chapter headers

When a chapter starts, a bordered header is displayed:

```
┌──────────┐
│  Setup   │
└──────────┘
```

## Navigation

At each Enter prompt (when waiting for the next step), you can type navigation
commands instead of just pressing Enter:

| Input    | Action                              |
|----------|-------------------------------------|
| Enter    | Advance to the next step (normal)   |
| `n`      | Skip to the start of the next chapter |
| `p`      | Jump back to the start of the previous chapter |
| `1`-`9`  | Jump directly to chapter N (1-indexed) |

This is especially useful during conference talks or workshops where you might
need to skip ahead or revisit a section.

## Chapters with other features

All step types work inside chapters — commands, comments, directives,
conditionals, fake output, etc:

```yaml
chapters:
  - name: "Environment Check"
    steps:
      - comment: "Checking if Docker is available..."
      - text: "which docker"
        capture:
          name: has_docker
          pattern: "(.*)"
      - text: "docker --version"
        if: has_docker

  - name: "Build"
    steps:
      - text: "docker build -t myapp ."
        if: has_docker
      - comment: "Build complete!"
```

## Tips

- Keep chapter names short — they appear in a bordered box
- Use chapters for demos longer than 5-6 steps
- Combine with `--dry-run` to preview the chapter structure before presenting
- Navigation only works in interactive mode (not with `auto_advance`)
