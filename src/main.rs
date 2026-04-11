use clap::Parser;
use rand::Rng;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};
use std::thread;
use std::time::Duration;

#[derive(Deserialize)]
struct Config {
    #[serde(default = "default_prompt")]
    prompt: String,
    #[serde(default)]
    clear: bool,
    #[serde(default)]
    speed: Option<u64>,
    #[serde(default = "default_delay")]
    delay: u64,
    #[serde(default = "default_jitter")]
    jitter: u64,
    #[serde(default = "default_pause")]
    pause: u64,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
struct Capture {
    name: String,
    pattern: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Step {
    Directive(String),
    Command(CommandStep),
}

#[derive(Deserialize)]
struct CommandStep {
    text: String,
    #[serde(default)]
    speed: Option<u64>,
    #[serde(default)]
    delay: Option<u64>,
    #[serde(default)]
    jitter: Option<u64>,
    #[serde(default)]
    pause: Option<u64>,
    #[serde(default)]
    capture: Option<Capture>,
}

fn default_prompt() -> String {
    "{green}~{reset} {blue}${reset} ".to_string()
}

fn expand_colors(s: &str) -> String {
    s.replace("{black}", "\x1B[30m")
        .replace("{red}", "\x1B[31m")
        .replace("{green}", "\x1B[32m")
        .replace("{yellow}", "\x1B[33m")
        .replace("{blue}", "\x1B[34m")
        .replace("{magenta}", "\x1B[35m")
        .replace("{cyan}", "\x1B[36m")
        .replace("{white}", "\x1B[37m")
        .replace("{bold}", "\x1B[1m")
        .replace("{dim}", "\x1B[2m")
        .replace("{reset}", "\x1B[0m")
}

fn default_delay() -> u64 {
    50
}
fn default_jitter() -> u64 {
    40
}
fn default_pause() -> u64 {
    200
}

fn speed_to_delay(speed: u64) -> u64 {
    if speed == 0 {
        return default_delay();
    }

    ((1000.0 / speed as f64).round() as u64).max(1)
}

fn resolve_delay(step: &CommandStep, config: &Config) -> u64 {
    if let Some(speed) = step.speed.or(config.speed) {
        return speed_to_delay(speed);
    }

    step.delay.unwrap_or(config.delay)
}

#[derive(Parser)]
#[command(name = "demonator", about = "Typewriter-style text display for terminal demos")]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "demo.yml")]
    config: PathBuf,
}

fn type_text(text: &str, delay: u64, jitter: u64, pause: u64) {
    let mut rng = rand::thread_rng();
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for ch in text.chars() {
        let base = delay as f64;
        let jitter_amount = if jitter > 0 {
            let j = (base * jitter as f64) / 100.0;
            rng.gen_range(-j..j)
        } else {
            0.0
        };

        let mut sleep_ms = (base + jitter_amount).max(5.0) as u64;

        if matches!(ch, '.' | ',' | ';' | ':' | '!' | '?') {
            sleep_ms += pause;
        }

        thread::sleep(Duration::from_millis(sleep_ms));

        handle.write_all(ch.to_string().as_bytes()).unwrap();
        handle.flush().unwrap();
    }

    handle.flush().unwrap();
}

fn wait_for_enter() {
    let tty = fs::File::open("/dev/tty").expect("failed to open /dev/tty");
    let mut reader = io::BufReader::new(tty);
    let mut buf = [0u8; 1];
    loop {
        if reader.read(&mut buf).unwrap_or(0) > 0 && buf[0] == b'\n' {
            break;
        }
    }
}

fn substitute_vars(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (name, value) in vars {
        result = result.replace(&format!("{{{}}}", name), value);
    }
    result
}

fn run_command(cmd: &str, capture: Option<&Capture>) -> Option<String> {
    let needs_capture = capture.is_some();

    if needs_capture {
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);
                print!("{}", stdout);
                eprint!("{}", stderr);
                io::stdout().flush().unwrap();
                io::stderr().flush().unwrap();

                if !o.status.success() {
                    if let Some(code) = o.status.code() {
                        eprintln!("[demonator] command exited with status {}", code);
                    }
                }

                if let Some(cap) = capture {
                    if let Ok(re) = Regex::new(&cap.pattern) {
                        // Try stdout first, then stderr
                        let combined = format!("{}{}", stdout, stderr);
                        if let Some(caps) = re.captures(&combined) {
                            if let Some(m) = caps.get(1) {
                                return Some(m.as_str().to_string());
                            }
                        }
                    } else {
                        eprintln!("[demonator] invalid capture pattern: {}", cap.pattern);
                    }
                }
            }
            Err(e) => {
                eprintln!("[demonator] failed to run command: {}", e);
            }
        }
    } else {
        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

        match status {
            Ok(s) => {
                if !s.success() {
                    if let Some(code) = s.code() {
                        eprintln!("[demonator] command exited with status {}", code);
                    }
                }
            }
            Err(e) => {
                eprintln!("[demonator] failed to run command: {}", e);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_colors_basic() {
        let result = expand_colors("{red}hello{reset}");
        assert_eq!(result, "\x1B[31mhello\x1B[0m");
    }

    #[test]
    fn test_expand_colors_all_colors() {
        assert_eq!(expand_colors("{black}"), "\x1B[30m");
        assert_eq!(expand_colors("{red}"), "\x1B[31m");
        assert_eq!(expand_colors("{green}"), "\x1B[32m");
        assert_eq!(expand_colors("{yellow}"), "\x1B[33m");
        assert_eq!(expand_colors("{blue}"), "\x1B[34m");
        assert_eq!(expand_colors("{magenta}"), "\x1B[35m");
        assert_eq!(expand_colors("{cyan}"), "\x1B[36m");
        assert_eq!(expand_colors("{white}"), "\x1B[37m");
        assert_eq!(expand_colors("{bold}"), "\x1B[1m");
        assert_eq!(expand_colors("{dim}"), "\x1B[2m");
        assert_eq!(expand_colors("{reset}"), "\x1B[0m");
    }

    #[test]
    fn test_expand_colors_no_placeholders() {
        assert_eq!(expand_colors("plain text"), "plain text");
    }

    #[test]
    fn test_expand_colors_multiple() {
        let result = expand_colors("{green}~{reset} {blue}${reset}");
        assert_eq!(result, "\x1B[32m~\x1B[0m \x1B[34m$\x1B[0m");
    }

    #[test]
    fn test_default_values() {
        assert_eq!(default_delay(), 50);
        assert_eq!(default_jitter(), 40);
        assert_eq!(default_pause(), 200);
        assert_eq!(default_prompt(), "{green}~{reset} {blue}${reset} ");
        assert_eq!(speed_to_delay(20), 50);
    }

    #[test]
    fn test_config_deserialize_minimal() {
        let yaml = "steps:\n  - text: 'echo hello'\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.steps.len(), 1);
        match &config.steps[0] {
            Step::Command(cmd) => assert_eq!(cmd.text, "echo hello"),
            _ => panic!("expected Command step"),
        }
        assert_eq!(config.speed, None);
        assert_eq!(config.delay, 50);
        assert_eq!(config.jitter, 40);
        assert_eq!(config.pause, 200);
        assert!(!config.clear);
        assert_eq!(config.prompt, "{green}~{reset} {blue}${reset} ");
    }

    #[test]
    fn test_config_deserialize_full() {
        let yaml = r#"
speed: 12
delay: 100
jitter: 20
pause: 500
clear: true
prompt: "$ "
steps:
  - text: "echo hello"
  - text: "ls -la"
    speed: 25
    delay: 30
    jitter: 10
    pause: 100
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.speed, Some(12));
        assert_eq!(config.delay, 100);
        assert_eq!(config.jitter, 20);
        assert_eq!(config.pause, 500);
        assert!(config.clear);
        assert_eq!(config.prompt, "$ ");
        assert_eq!(config.steps.len(), 2);
        match &config.steps[1] {
            Step::Command(cmd) => {
                assert_eq!(cmd.speed, Some(25));
                assert_eq!(cmd.delay, Some(30));
                assert_eq!(cmd.jitter, Some(10));
                assert_eq!(cmd.pause, Some(100));
            }
            _ => panic!("expected Command step"),
        }
    }

    #[test]
    fn test_step_defaults_to_none() {
        let yaml = "text: 'echo hello'\n";
        let step: Step = serde_yaml::from_str(yaml).unwrap();
        match step {
            Step::Command(cmd) => {
                assert_eq!(cmd.text, "echo hello");
                assert!(cmd.speed.is_none());
                assert!(cmd.delay.is_none());
                assert!(cmd.jitter.is_none());
                assert!(cmd.pause.is_none());
            }
            _ => panic!("expected Command step"),
        }
    }

    #[test]
    fn test_step_override_resolves_correctly() {
        let yaml = r#"
delay: 80
jitter: 30
pause: 300
steps:
  - text: "cmd1"
  - text: "cmd2"
    delay: 10
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        match (&config.steps[0], &config.steps[1]) {
            (Step::Command(s0), Step::Command(s1)) => {
                assert_eq!(resolve_delay(s0, &config), 80);
                assert_eq!(s0.jitter.unwrap_or(config.jitter), 30);
                assert_eq!(s0.pause.unwrap_or(config.pause), 300);

                assert_eq!(resolve_delay(s1, &config), 10);
                assert_eq!(s1.jitter.unwrap_or(config.jitter), 30);
            }
            _ => panic!("expected Command steps"),
        }
    }

    #[test]
    fn test_speed_override_resolves_correctly() {
        let yaml = r#"
speed: 20
delay: 80
jitter: 30
pause: 300
steps:
  - text: "cmd1"
  - text: "cmd2"
    speed: 40
  - text: "cmd3"
    delay: 10
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        match (&config.steps[0], &config.steps[1], &config.steps[2]) {
            (Step::Command(s0), Step::Command(s1), Step::Command(s2)) => {
                assert_eq!(resolve_delay(s0, &config), 50);
                assert_eq!(resolve_delay(s1, &config), 25);
                assert_eq!(resolve_delay(s2, &config), 50);
            }
            _ => panic!("expected Command steps"),
        }
    }

    #[test]
    fn test_config_empty_steps() {
        let yaml = "steps: []\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.steps.is_empty());
    }

    #[test]
    fn test_config_missing_steps_fails() {
        let yaml = "delay: 50\n";
        let result: Result<Config, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_substitute_vars_basic() {
        let mut vars = HashMap::new();
        vars.insert("session_id".to_string(), "abc123".to_string());
        assert_eq!(
            substitute_vars("nono attach {session_id}", &vars),
            "nono attach abc123"
        );
    }

    #[test]
    fn test_substitute_vars_multiple() {
        let mut vars = HashMap::new();
        vars.insert("host".to_string(), "localhost".to_string());
        vars.insert("port".to_string(), "8080".to_string());
        assert_eq!(
            substitute_vars("curl {host}:{port}", &vars),
            "curl localhost:8080"
        );
    }

    #[test]
    fn test_substitute_vars_no_match() {
        let vars = HashMap::new();
        assert_eq!(substitute_vars("no vars here", &vars), "no vars here");
    }

    #[test]
    fn test_substitute_vars_does_not_replace_colors() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "world".to_string());
        // Color placeholders like {red} should not be affected unless a var named "red" exists
        assert_eq!(
            substitute_vars("{red}hello {name}{reset}", &vars),
            "{red}hello world{reset}"
        );
    }

    #[test]
    fn test_capture_deserialize() {
        let yaml = r#"
steps:
  - text: "echo hello"
    capture:
      name: session_id
      pattern: "session (\\w+)"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        match &config.steps[0] {
            Step::Command(cmd) => {
                let cap = cmd.capture.as_ref().unwrap();
                assert_eq!(cap.name, "session_id");
                assert_eq!(cap.pattern, "session (\\w+)");
            }
            _ => panic!("expected Command step"),
        }
    }

    #[test]
    fn test_capture_regex_extraction() {
        let pattern = r"Started detached session (\w+)";
        let text = "Started detached session e96400cf26349136.\nAttach with: nono attach e96400cf26349136\n";
        let re = Regex::new(pattern).unwrap();
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "e96400cf26349136");
    }

    #[test]
    fn test_pause_directive() {
        let yaml = r#"
steps:
  - pause
  - text: "echo hello"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.steps.len(), 2);
        match &config.steps[0] {
            Step::Directive(d) => assert_eq!(d, "pause"),
            _ => panic!("expected Directive step"),
        }
        match &config.steps[1] {
            Step::Command(cmd) => assert_eq!(cmd.text, "echo hello"),
            _ => panic!("expected Command step"),
        }
    }

    #[test]
    fn test_clear_directive() {
        let yaml = r#"
steps:
  - text: "echo hello"
  - clear
  - text: "echo world"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.steps.len(), 3);
        match &config.steps[1] {
            Step::Directive(d) => assert_eq!(d, "clear"),
            _ => panic!("expected Directive step"),
        }
    }

    #[test]
    fn test_step_without_capture() {
        let yaml = r#"
steps:
  - text: "echo hello"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        match &config.steps[0] {
            Step::Command(cmd) => assert!(cmd.capture.is_none()),
            _ => panic!("expected Command step"),
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let contents = match fs::read_to_string(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {}: {}", cli.config.display(), e);
            process::exit(1);
        }
    };

    let config: Config = match serde_yaml::from_str(&contents) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error parsing {}: {}", cli.config.display(), e);
            process::exit(1);
        }
    };

    if config.clear {
        print!("\x1B[2J\x1B[H");
        io::stdout().flush().unwrap();
    }

    let mut vars: HashMap<String, String> = HashMap::new();

    for step in &config.steps {
        match step {
            Step::Directive(directive) => match directive.as_str() {
                "pause" => {
                    print!("{}", expand_colors(&config.prompt));
                    io::stdout().flush().unwrap();
                    wait_for_enter();
                }
                "clear" => {
                    print!("\x1B[2J\x1B[H");
                    io::stdout().flush().unwrap();
                }
                other => {
                    eprintln!("[demonator] unknown directive: {}", other);
                    process::exit(1);
                }
            },
            Step::Command(step) => {
                let delay = resolve_delay(step, &config);
                let jitter = step.jitter.unwrap_or(config.jitter);
                let pause = step.pause.unwrap_or(config.pause);

                let cmd = substitute_vars(&step.text, &vars);

                print!("{}", expand_colors(&config.prompt));
                io::stdout().flush().unwrap();
                type_text(&cmd, delay, jitter, pause);
                wait_for_enter();

                if let Some(captured) = run_command(&cmd, step.capture.as_ref()) {
                    if let Some(ref cap) = step.capture {
                        vars.insert(cap.name.clone(), captured);
                    }
                }
            }
        }
    }
}
