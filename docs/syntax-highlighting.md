# Syntax Highlighting

Syntax highlighting colorizes commands as they are typed, making them easier to
read during a demo.

## Enabling highlighting

Set `highlight: true` at the global level:

```yaml
highlight: true

steps:
  - text: "cat README.md | grep -i 'install' > output.txt"
  - text: "docker build --tag myapp:latest ."
  - text: "echo $HOME"
  - text: "curl -s 'https://api.example.com/data' | jq '.results[]'"
```

## Color scheme

| Element                      | Color        | Example               |
|------------------------------|--------------|-----------------------|
| Command (first word)         | Bold white   | `docker`, `echo`      |
| Flags (`--foo`, `-v`)        | Yellow       | `--tag`, `-s`         |
| Quoted strings (`"`, `'`)    | Green        | `"hello"`, `'*.txt'`  |
| Pipes and redirects          | Cyan         | `\|`, `>`, `>>`       |
| Operators (`&&`, `;`)        | Cyan         | `&&`, `;`             |
| Variables (`$FOO`, `${BAR}`) | Magenta      | `$HOME`, `${PATH}`    |
| Shell comments               | Green        | `# this is a comment` |
| Everything else              | Default      | arguments, paths      |

## How it works

Highlighting is applied character-by-character as the command is typed. Each
character is emitted with the appropriate ANSI color code, so the color appears
immediately — there's no flicker or delay.

After a pipe (`|`), semicolon (`;`), or double-ampersand (`&&`), the next word
is treated as a new command and highlighted in bold white.

## Example output

The command:

```
cat data.json | grep -i "error" > errors.txt && echo "done"
```

Would appear as:

- **cat** (bold white) data.json (default) **|** (cyan) **grep** (bold white) **-i** (yellow) **"error"** (green) **>** (cyan) errors.txt (default) **&&** (cyan) **echo** (bold white) **"done"** (green)

## Tips

- Highlighting works with all other features (fake output typing, auto-advance,
  chapters, etc.)
- The highlighter handles nested quotes, escaped characters in double-quoted
  strings, and `${brace}` variable syntax
- For commands that are mostly flags and strings (like `curl` or `docker`),
  highlighting significantly improves readability
