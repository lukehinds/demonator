# User Prompts

User prompt steps pause the demo and collect input from the presenter. The
response is stored as a variable that subsequent steps can reference or use as
a condition.

## Yes/no confirmation (`ask`)

Displays a question and waits for `y` or `n`. If the user answers yes, the
capture variable is set to `"y"`. If they answer no (or just press Enter), the
variable is removed.

```yaml
steps:
  - ask: "Deploy to production?"
    capture: confirmed

  - text: "kubectl apply -f deployment.yaml"
    if: confirmed

  - comment: "Skipping deployment."
    unless: confirmed
```

The prompt is displayed as:

```
? Deploy to production? [y/N]
```

Accepted yes answers: `y`, `yes`. Everything else is treated as no.

## Free-text input (`input`)

Displays a question and reads a line of text from the presenter. The value is
stored in the capture variable. Press Enter with no input to use the default
(if one is set) or leave the variable unset.

```yaml
steps:
  - input: "Enter a profile name:"
    capture: profile_name
    default: "default-local"

  - text: "nono profile save {profile_name}"
    if: profile_name
```

The prompt is displayed with a hint in brackets:

```
? Enter a profile name: [default-local]
```

When no `default` is set, the hint shows `[enter]` to indicate that pressing
Enter skips the step:

```
? Enter a name (or press Enter to skip): [enter]
```

### Fields

| Field     | Required | Description                                           |
|-----------|----------|-------------------------------------------------------|
| `input`   | yes      | The question to display                               |
| `capture` | yes      | Variable name to store the response in                |
| `default` | no       | Value to use when the user presses Enter without typing |

## Combining with conditionals

Both step types integrate directly with `if:` and `unless:` on subsequent
steps, since they store values into the same variable map used by captured
command output.

```yaml
steps:
  - ask: "Run database migrations?"
    capture: run_migrations

  - input: "Target environment (staging/production):"
    capture: env
    default: "staging"

  - text: "flyway -url=jdbc:{env} migrate"
    if: run_migrations

  - comment: "Skipping migrations."
    unless: run_migrations
```

## Dry-run display

In `--dry-run` mode, prompt steps are annotated with the capture variable name:

```
? Deploy to production? [y/N]  → confirmed
? Enter a profile name: [default-local]  → profile_name
```
