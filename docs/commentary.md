# Commentary and Narration

Commentary blocks let you add explanatory text between commands. They appear
without a prompt prefix and are not executed — they're purely for narration.

## Basic usage

```yaml
steps:
  - comment: "First, let's clone the repository and build the project."
  - text: "git clone git@github.com:example/myproject.git"
  - text: "cd myproject"

  - comment: "Now we'll run the test suite to make sure everything passes."
  - text: "cargo test"

  - comment: "All green! Let's build for release."
  - text: "cargo build --release"
```

Comments are rendered in **dim** text by default, visually separating them from
the commands and their output.

## Styling

Use the `style` field to change the appearance:

```yaml
- comment: "This is dim (default)"

- comment: "This is bold"
  style: bold

- comment: "This is italic"
  style: italic

- comment: "This is in yellow"
  style: yellow
```

Available styles:

| Style      | Effect                    |
|------------|---------------------------|
| `dim`      | Dimmed text (default)     |
| `bold`     | Bold text                 |
| `italic`   | Italic text               |
| `red`      | Red text                  |
| `green`    | Green text                |
| `yellow`   | Yellow text               |
| `blue`     | Blue text                 |
| `magenta`  | Magenta text              |
| `cyan`     | Cyan text                 |

## Tips

- Use comments to explain **why** you're running a command, not just what it does
- Keep comments short — the audience is watching a live demo, not reading docs
- Comments appear instantly (no typewriter effect) so they don't slow down the flow
- Combine with the `pause` directive if you want to give the audience time to read:

```yaml
- comment: "This next part deploys to production. Watch closely."
- pause
- text: "kubectl apply -f deploy.yml"
```
