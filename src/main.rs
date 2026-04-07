use clap::Parser;
use rand::Rng;
use serde::Deserialize;
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
struct Step {
    text: String,
    #[serde(default)]
    speed: Option<u64>,
    #[serde(default)]
    delay: Option<u64>,
    #[serde(default)]
    jitter: Option<u64>,
    #[serde(default)]
    pause: Option<u64>,
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

fn resolve_delay(step: &Step, config: &Config) -> u64 {
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
        handle.write_all(ch.to_string().as_bytes()).unwrap();
        handle.flush().unwrap();

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

fn run_command(cmd: &str) {
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
        assert_eq!(config.steps[0].text, "echo hello");
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
        assert_eq!(config.steps[1].speed, Some(25));
        assert_eq!(config.steps[1].delay, Some(30));
        assert_eq!(config.steps[1].jitter, Some(10));
        assert_eq!(config.steps[1].pause, Some(100));
    }

    #[test]
    fn test_step_defaults_to_none() {
        let yaml = "text: 'echo hello'\n";
        let step: Step = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(step.text, "echo hello");
        assert!(step.speed.is_none());
        assert!(step.delay.is_none());
        assert!(step.jitter.is_none());
        assert!(step.pause.is_none());
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
        let s0 = &config.steps[0];
        let s1 = &config.steps[1];

        assert_eq!(resolve_delay(s0, &config), 80);
        assert_eq!(s0.jitter.unwrap_or(config.jitter), 30);
        assert_eq!(s0.pause.unwrap_or(config.pause), 300);

        assert_eq!(resolve_delay(s1, &config), 10);
        assert_eq!(s1.jitter.unwrap_or(config.jitter), 30);
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
        let s0 = &config.steps[0];
        let s1 = &config.steps[1];
        let s2 = &config.steps[2];

        assert_eq!(resolve_delay(s0, &config), 50);
        assert_eq!(resolve_delay(s1, &config), 25);
        assert_eq!(resolve_delay(s2, &config), 50);
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

    for step in &config.steps {
        let delay = resolve_delay(step, &config);
        let jitter = step.jitter.unwrap_or(config.jitter);
        let pause = step.pause.unwrap_or(config.pause);

        print!("{}", expand_colors(&config.prompt));
        io::stdout().flush().unwrap();
        type_text(&step.text, delay, jitter, pause);
        wait_for_enter();
        run_command(&step.text);
    }
}
