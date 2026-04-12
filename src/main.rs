use clap::Parser;
use rand::Rng;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

// ---------------------------------------------------------------------------
// Configuration structures
// ---------------------------------------------------------------------------

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
    #[serde(default)]
    highlight: bool,
    #[serde(default)]
    auto_advance: Option<u64>,
    #[serde(default)]
    setup: Option<Vec<String>>,
    #[serde(default)]
    teardown: Option<Vec<String>>,
    #[serde(default)]
    steps: Vec<Step>,
    #[serde(default)]
    chapters: Vec<Chapter>,
}

#[derive(Deserialize)]
struct Chapter {
    name: String,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Step {
    Directive(String),
    Command(CommandStep),
    Comment(CommentStep),
}

#[derive(Deserialize)]
struct CommentStep {
    comment: String,
    #[serde(default)]
    style: Option<String>,
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
    #[serde(default)]
    fake_output: Option<String>,
    #[serde(default)]
    output_speed: Option<u64>,
    #[serde(default = "default_true")]
    execute: bool,
    #[serde(default)]
    wait_for: Option<String>,
    #[serde(default = "default_timeout")]
    timeout: u64,
    #[serde(default)]
    wait: Option<u64>,
    #[serde(default)]
    interact: Option<Vec<Interaction>>,
    #[serde(default, rename = "if")]
    if_condition: Option<String>,
    #[serde(default)]
    unless: Option<String>,
}

#[derive(Deserialize)]
struct Capture {
    name: String,
    pattern: String,
}

#[derive(Deserialize)]
struct Interaction {
    expect: String,
    send: String,
}

fn default_true() -> bool {
    true
}
fn default_timeout() -> u64 {
    30
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "demonator", about = "Typewriter-style text display for terminal demos")]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "demo.yml")]
    config: PathBuf,

    /// Preview demo flow without executing or animating
    #[arg(long)]
    dry_run: bool,

    /// Re-run demo when config file changes
    #[arg(long)]
    watch: bool,
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

fn default_prompt() -> String {
    "{green}~{reset} {blue}${reset} ".to_string()
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

// ---------------------------------------------------------------------------
// Color / highlighting
// ---------------------------------------------------------------------------

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

const HL_RESET: &str = "\x1B[0m";
const HL_BOLD_WHITE: &str = "\x1B[1;37m";
const HL_YELLOW: &str = "\x1B[33m";
const HL_GREEN: &str = "\x1B[32m";
const HL_CYAN: &str = "\x1B[36m";
const HL_MAGENTA: &str = "\x1B[35m";

fn highlight_command(text: &str) -> Vec<(char, &'static str)> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut result = Vec::with_capacity(len);
    let mut i = 0;
    let mut is_first_word = true;

    while i < len {
        let ch = chars[i];

        if ch == ' ' || ch == '\t' {
            result.push((ch, HL_RESET));
            i += 1;
            continue;
        }

        // Pipe, semicolon, ampersand — operators
        if ch == '|' || ch == ';' {
            result.push((ch, HL_CYAN));
            i += 1;
            is_first_word = true;
            continue;
        }
        if ch == '&' {
            result.push((ch, HL_CYAN));
            i += 1;
            if i < len && chars[i] == '&' {
                result.push(('&', HL_CYAN));
                i += 1;
            }
            is_first_word = true;
            continue;
        }

        // Redirects
        if ch == '>' || ch == '<' {
            result.push((ch, HL_CYAN));
            i += 1;
            if i < len && (chars[i] == '>' || chars[i] == '&') {
                result.push((chars[i], HL_CYAN));
                i += 1;
            }
            continue;
        }

        // Quoted strings
        if ch == '"' || ch == '\'' {
            let quote = ch;
            result.push((ch, HL_GREEN));
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' && quote == '"' {
                    result.push((chars[i], HL_GREEN));
                    i += 1;
                    if i < len {
                        result.push((chars[i], HL_GREEN));
                        i += 1;
                    }
                } else {
                    result.push((chars[i], HL_GREEN));
                    i += 1;
                }
            }
            if i < len {
                result.push((chars[i], HL_GREEN));
                i += 1;
            }
            is_first_word = false;
            continue;
        }

        // Variables
        if ch == '$' {
            result.push((ch, HL_MAGENTA));
            i += 1;
            if i < len && chars[i] == '{' {
                while i < len {
                    let c = chars[i];
                    result.push((c, HL_MAGENTA));
                    i += 1;
                    if c == '}' {
                        break;
                    }
                }
            } else if i < len && chars[i] == '(' {
                result.push((chars[i], HL_MAGENTA));
                i += 1;
                // Subshell — just color the parens, let inner parse normally
            } else {
                while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    result.push((chars[i], HL_MAGENTA));
                    i += 1;
                }
            }
            is_first_word = false;
            continue;
        }

        // Flags: -x, --long-flag
        if ch == '-' && (i == 0 || chars[i - 1] == ' ' || chars[i - 1] == '\t') {
            while i < len && chars[i] != ' ' && chars[i] != '\t' && chars[i] != '=' {
                result.push((chars[i], HL_YELLOW));
                i += 1;
            }
            // Include = sign if present
            if i < len && chars[i] == '=' {
                result.push((chars[i], HL_YELLOW));
                i += 1;
            }
            is_first_word = false;
            continue;
        }

        // Comments in shell
        if ch == '#' && (i == 0 || chars[i - 1] == ' ') {
            while i < len {
                result.push((chars[i], HL_GREEN));
                i += 1;
            }
            continue;
        }

        // Regular word
        let color = if is_first_word {
            HL_BOLD_WHITE
        } else {
            HL_RESET
        };
        while i < len
            && !matches!(
                chars[i],
                ' ' | '\t' | '|' | ';' | '&' | '>' | '<' | '"' | '\'' | '$'
            )
        {
            result.push((chars[i], color));
            i += 1;
        }
        is_first_word = false;
    }

    result
}

// ---------------------------------------------------------------------------
// Timing
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Text output
// ---------------------------------------------------------------------------

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

fn type_text_highlighted(tokens: &[(char, &str)], delay: u64, jitter: u64, pause: u64) {
    let mut rng = rand::thread_rng();
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let mut current_color = "";

    for &(ch, color) in tokens {
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

        if color != current_color {
            handle.write_all(color.as_bytes()).unwrap();
            current_color = color;
        }
        handle.write_all(ch.to_string().as_bytes()).unwrap();
        handle.flush().unwrap();
    }

    // Reset color at end
    handle.write_all(HL_RESET.as_bytes()).unwrap();
    handle.flush().unwrap();
}

// ---------------------------------------------------------------------------
// Input handling
// ---------------------------------------------------------------------------

enum NavAction {
    Continue,
    NextChapter,
    PrevChapter,
    JumpChapter(usize),
}

fn wait_for_input(has_chapters: bool) -> NavAction {
    let tty = fs::File::open("/dev/tty").expect("failed to open /dev/tty");
    let mut reader = io::BufReader::new(tty);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap_or(0);
    let trimmed = line.trim();

    if !has_chapters || trimmed.is_empty() {
        return NavAction::Continue;
    }

    match trimmed {
        "n" => NavAction::NextChapter,
        "p" => NavAction::PrevChapter,
        s => {
            if let Ok(num) = s.parse::<usize>() {
                if num > 0 {
                    NavAction::JumpChapter(num - 1)
                } else {
                    NavAction::Continue
                }
            } else {
                NavAction::Continue
            }
        }
    }
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

// ---------------------------------------------------------------------------
// Variable substitution
// ---------------------------------------------------------------------------

fn substitute_vars(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (name, value) in vars {
        result = result.replace(&format!("{{{}}}", name), value);
    }
    result
}

// ---------------------------------------------------------------------------
// Command execution
// ---------------------------------------------------------------------------

fn run_command(cmd: &str, capture: Option<&Capture>) -> (Option<String>, i32) {
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
                let stdout_str = String::from_utf8_lossy(&o.stdout);
                let stderr_str = String::from_utf8_lossy(&o.stderr);
                print!("{}", stdout_str);
                eprint!("{}", stderr_str);
                io::stdout().flush().unwrap();
                io::stderr().flush().unwrap();

                let code = o.status.code().unwrap_or(1);
                if !o.status.success() {
                    eprintln!("[demonator] command exited with status {}", code);
                }

                if let Some(cap) = capture {
                    if let Ok(re) = Regex::new(&cap.pattern) {
                        let combined = format!("{}{}", stdout_str, stderr_str);
                        if let Some(caps) = re.captures(&combined) {
                            if let Some(m) = caps.get(1) {
                                return (Some(m.as_str().to_string()), code);
                            }
                        }
                    } else {
                        eprintln!("[demonator] invalid capture pattern: {}", cap.pattern);
                    }
                }

                (None, code)
            }
            Err(e) => {
                eprintln!("[demonator] failed to run command: {}", e);
                (None, 1)
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
                let code = s.code().unwrap_or(1);
                if !s.success() {
                    eprintln!("[demonator] command exited with status {}", code);
                }
                (None, code)
            }
            Err(e) => {
                eprintln!("[demonator] failed to run command: {}", e);
                (None, 1)
            }
        }
    }
}

fn run_command_wait_for(cmd: &str, pattern: &str, timeout_secs: u64) -> i32 {
    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[demonator] invalid wait_for pattern: {}", e);
            return 1;
        }
    };

    let mut child = match Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[demonator] failed to run command: {}", e);
            return 1;
        }
    };

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    let tx2 = tx.clone();
    thread::spawn(move || {
        let reader = io::BufReader::new(stdout);
        for line in reader.split(b'\n') {
            if let Ok(bytes) = line {
                let mut with_nl = bytes;
                with_nl.push(b'\n');
                if tx.send(with_nl).is_err() {
                    break;
                }
            }
        }
    });

    thread::spawn(move || {
        let reader = io::BufReader::new(stderr);
        for line in reader.split(b'\n') {
            if let Ok(bytes) = line {
                let mut with_nl = bytes;
                with_nl.push(b'\n');
                if tx2.send(with_nl).is_err() {
                    break;
                }
            }
        }
    });

    let deadline = SystemTime::now() + Duration::from_secs(timeout_secs);
    let mut accumulated = String::new();

    loop {
        let remaining = deadline
            .duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO);
        if remaining.is_zero() {
            eprintln!(
                "[demonator] wait_for timed out after {}s",
                timeout_secs
            );
            let _ = child.kill();
            let _ = child.wait();
            return 1;
        }

        match rx.recv_timeout(remaining.min(Duration::from_millis(100))) {
            Ok(chunk) => {
                let text = String::from_utf8_lossy(&chunk);
                print!("{}", text);
                io::stdout().flush().unwrap();
                accumulated.push_str(&text);

                if re.is_match(&accumulated) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return 0;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Check if child exited
                if let Ok(Some(status)) = child.try_wait() {
                    // Drain remaining output
                    for chunk in rx.try_iter() {
                        let text = String::from_utf8_lossy(&chunk);
                        print!("{}", text);
                        accumulated.push_str(&text);
                    }
                    io::stdout().flush().unwrap();

                    if re.is_match(&accumulated) {
                        return 0;
                    }
                    eprintln!("[demonator] command exited before pattern matched");
                    return status.code().unwrap_or(1);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = child.wait();
                if re.is_match(&accumulated) {
                    return 0;
                }
                eprintln!("[demonator] command exited before pattern matched");
                return 1;
            }
        }
    }
}

fn run_command_interact(cmd: &str, interactions: &[Interaction]) -> i32 {
    let mut child = match Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[demonator] failed to run command: {}", e);
            return 1;
        }
    };

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    thread::spawn(move || {
        let mut reader = io::BufReader::new(stdout);
        let mut buf = [0u8; 256];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut accumulated = String::new();
    let mut interaction_idx = 0;

    while interaction_idx < interactions.len() {
        match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(chunk) => {
                let text = String::from_utf8_lossy(&chunk);
                print!("{}", text);
                io::stdout().flush().unwrap();
                accumulated.push_str(&text);

                if accumulated.contains(&interactions[interaction_idx].expect) {
                    let response = format!("{}\n", interactions[interaction_idx].send);
                    if stdin.write_all(response.as_bytes()).is_err() {
                        break;
                    }
                    let _ = stdin.flush();
                    accumulated.clear();
                    interaction_idx += 1;
                }
            }
            Err(_) => break,
        }
    }

    // Drain remaining output
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(chunk) => {
                print!("{}", String::from_utf8_lossy(&chunk));
            }
            Err(_) => break,
        }
    }
    io::stdout().flush().unwrap();

    let status = child.wait().unwrap_or_else(|_| process::exit(1));
    status.code().unwrap_or(1)
}

// ---------------------------------------------------------------------------
// Feature helpers
// ---------------------------------------------------------------------------

fn should_run_step(cmd: &CommandStep, vars: &HashMap<String, String>) -> bool {
    if let Some(ref var_name) = cmd.if_condition {
        match vars.get(var_name) {
            Some(v) if !v.trim().is_empty() => {}
            _ => return false,
        }
    }
    if let Some(ref var_name) = cmd.unless {
        if let Some(v) = vars.get(var_name) {
            if !v.trim().is_empty() {
                return false;
            }
        }
    }
    true
}

fn style_to_ansi(style: Option<&str>) -> &str {
    match style {
        Some("dim") => "\x1B[2m",
        Some("bold") => "\x1B[1m",
        Some("italic") => "\x1B[3m",
        Some("red") => "\x1B[31m",
        Some("green") => "\x1B[32m",
        Some("yellow") => "\x1B[33m",
        Some("blue") => "\x1B[34m",
        Some("magenta") => "\x1B[35m",
        Some("cyan") => "\x1B[36m",
        _ => "\x1B[2m", // default to dim
    }
}

fn print_comment(comment: &CommentStep) {
    let ansi = style_to_ansi(comment.style.as_deref());
    println!("{}{}\x1B[0m", ansi, comment.comment);
}

fn print_chapter_header(name: &str) {
    let bar = "─".repeat(name.len() + 4);
    println!("\x1B[1;36m┌{}┐\x1B[0m", bar);
    println!("\x1B[1;36m│  {}  │\x1B[0m", name);
    println!("\x1B[1;36m└{}┘\x1B[0m", bar);
    println!();
}

fn run_hidden_commands(commands: &[String]) {
    for cmd in commands {
        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if let Ok(s) = status {
            if !s.success() {
                eprintln!(
                    "[demonator] setup/teardown command failed ({}): {}",
                    s.code().unwrap_or(-1),
                    cmd
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Demo resolution — flatten chapters into indexed steps
// ---------------------------------------------------------------------------

struct ChapterMarker {
    name: String,
    start: usize,
}

struct ResolvedDemo {
    steps: Vec<ResolvedStep>,
    chapters: Vec<ChapterMarker>,
}

struct ResolvedStep {
    step: StepRef,
}

enum StepRef {
    Directive(String),
    Comment(String, Option<String>),
    Command(CommandRef),
}

struct CommandRef {
    text: String,
    speed: Option<u64>,
    delay: Option<u64>,
    jitter: Option<u64>,
    pause: Option<u64>,
    capture: Option<CaptureRef>,
    fake_output: Option<String>,
    output_speed: Option<u64>,
    execute: bool,
    wait_for: Option<String>,
    timeout: u64,
    wait: Option<u64>,
    interact: Option<Vec<InteractionRef>>,
    if_condition: Option<String>,
    unless: Option<String>,
}

struct CaptureRef {
    name: String,
    pattern: String,
}

struct InteractionRef {
    expect: String,
    send: String,
}

fn resolve_step(step: &Step) -> ResolvedStep {
    match step {
        Step::Directive(d) => ResolvedStep {
            step: StepRef::Directive(d.clone()),
        },
        Step::Comment(c) => ResolvedStep {
            step: StepRef::Comment(c.comment.clone(), c.style.clone()),
        },
        Step::Command(cmd) => ResolvedStep {
            step: StepRef::Command(CommandRef {
                text: cmd.text.clone(),
                speed: cmd.speed,
                delay: cmd.delay,
                jitter: cmd.jitter,
                pause: cmd.pause,
                capture: cmd.capture.as_ref().map(|c| CaptureRef {
                    name: c.name.clone(),
                    pattern: c.pattern.clone(),
                }),
                fake_output: cmd.fake_output.clone(),
                output_speed: cmd.output_speed,
                execute: cmd.execute,
                wait_for: cmd.wait_for.clone(),
                timeout: cmd.timeout,
                wait: cmd.wait,
                interact: cmd.interact.as_ref().map(|v| {
                    v.iter()
                        .map(|i| InteractionRef {
                            expect: i.expect.clone(),
                            send: i.send.clone(),
                        })
                        .collect()
                }),
                if_condition: cmd.if_condition.clone(),
                unless: cmd.unless.clone(),
            }),
        },
    }
}

fn resolve_demo(config: &Config) -> ResolvedDemo {
    if !config.chapters.is_empty() {
        let mut steps = Vec::new();
        let mut chapters = Vec::new();

        for ch in &config.chapters {
            chapters.push(ChapterMarker {
                name: ch.name.clone(),
                start: steps.len(),
            });
            for s in &ch.steps {
                steps.push(resolve_step(s));
            }
        }

        ResolvedDemo { steps, chapters }
    } else {
        let steps = config.steps.iter().map(resolve_step).collect();
        ResolvedDemo {
            steps,
            chapters: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Dry-run display
// ---------------------------------------------------------------------------

fn print_dry_run(config: &Config) {
    let demo = resolve_demo(config);
    let prompt = expand_colors(&config.prompt);

    if let Some(ref setup) = config.setup {
        println!("\x1B[2m[setup]\x1B[0m");
        for cmd in setup {
            println!("  {}", cmd);
        }
        println!();
    }

    let mut chapter_idx = 0;

    for (i, resolved) in demo.steps.iter().enumerate() {
        // Check chapter boundary
        if chapter_idx < demo.chapters.len() && demo.chapters[chapter_idx].start == i {
            println!(
                "\x1B[1;36m── {} ──\x1B[0m",
                demo.chapters[chapter_idx].name
            );
            chapter_idx += 1;
        }

        match &resolved.step {
            StepRef::Directive(d) => {
                println!("\x1B[2m[{}]\x1B[0m", d);
            }
            StepRef::Comment(text, style) => {
                let ansi = style_to_ansi(style.as_deref());
                println!("{}{}\x1B[0m", ansi, text);
            }
            StepRef::Command(cmd) => {
                print!("{}{}", prompt, cmd.text);
                let mut annotations = Vec::new();
                if !cmd.execute {
                    annotations.push("no-exec");
                }
                if cmd.fake_output.is_some() {
                    annotations.push("fake-output");
                }
                if cmd.wait_for.is_some() {
                    annotations.push("wait-for");
                }
                if cmd.interact.is_some() {
                    annotations.push("interactive");
                }
                if cmd.if_condition.is_some() || cmd.unless.is_some() {
                    annotations.push("conditional");
                }
                if cmd.capture.is_some() {
                    annotations.push("capture");
                }
                if !annotations.is_empty() {
                    print!("  \x1B[2m[{}]\x1B[0m", annotations.join(", "));
                }
                println!();

                if let Some(ref fake) = cmd.fake_output {
                    for line in fake.lines() {
                        println!("  \x1B[2m▸ {}\x1B[0m", line);
                    }
                }
            }
        }
    }

    if let Some(ref teardown) = config.teardown {
        println!();
        println!("\x1B[2m[teardown]\x1B[0m");
        for cmd in teardown {
            println!("  {}", cmd);
        }
    }
}

// ---------------------------------------------------------------------------
// Config loading
// ---------------------------------------------------------------------------

fn load_config(path: &Path) -> Config {
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {}: {}", path.display(), e);
            process::exit(1);
        }
    };

    let config: Config = match serde_yaml::from_str(&contents) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error parsing {}: {}", path.display(), e);
            process::exit(1);
        }
    };

    if config.steps.is_empty() && config.chapters.is_empty() {
        eprintln!("Error: config must have either 'steps' or 'chapters'");
        process::exit(1);
    }
    if !config.steps.is_empty() && !config.chapters.is_empty() {
        eprintln!("Error: config cannot have both 'steps' and 'chapters'");
        process::exit(1);
    }

    config
}

// ---------------------------------------------------------------------------
// Main demo loop
// ---------------------------------------------------------------------------

fn run_demo(config: &Config, cli: &Cli) {
    // Setup
    if let Some(ref setup) = config.setup {
        run_hidden_commands(setup);
    }

    // Dry run
    if cli.dry_run {
        print_dry_run(config);
        return;
    }

    // Clear
    if config.clear {
        print!("\x1B[2J\x1B[H");
        io::stdout().flush().unwrap();
    }

    let demo = resolve_demo(config);
    let has_chapters = !demo.chapters.is_empty();
    let prompt = expand_colors(&config.prompt);

    let mut vars: HashMap<String, String> = HashMap::new();
    let mut idx: usize = 0;
    let mut chapter_idx: usize = 0;

    while idx < demo.steps.len() {
        // Check chapter boundary
        if chapter_idx < demo.chapters.len() && demo.chapters[chapter_idx].start == idx {
            print_chapter_header(&demo.chapters[chapter_idx].name);
            chapter_idx += 1;
        }

        match &demo.steps[idx].step {
            StepRef::Directive(directive) => match directive.as_str() {
                "pause" => {
                    print!("{}", prompt);
                    io::stdout().flush().unwrap();
                    if let Some(ms) = config.auto_advance {
                        thread::sleep(Duration::from_millis(ms));
                        println!();
                    } else {
                        wait_for_enter();
                    }
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

            StepRef::Comment(text, style) => {
                let comment = CommentStep {
                    comment: text.clone(),
                    style: style.clone(),
                };
                print_comment(&comment);
            }

            StepRef::Command(cmd) => {
                // Evaluate conditionals
                let should_run = {
                    let mut run = true;
                    if let Some(ref var_name) = cmd.if_condition {
                        match vars.get(var_name) {
                            Some(v) if !v.trim().is_empty() => {}
                            _ => run = false,
                        }
                    }
                    if let Some(ref var_name) = cmd.unless {
                        if let Some(v) = vars.get(var_name) {
                            if !v.trim().is_empty() {
                                run = false;
                            }
                        }
                    }
                    run
                };

                if !should_run {
                    idx += 1;
                    continue;
                }

                let resolved_text = substitute_vars(&cmd.text, &vars);

                // Type the command
                print!("{}", prompt);
                io::stdout().flush().unwrap();

                let base_delay = if let Some(speed) = cmd.speed.or(config.speed) {
                    speed_to_delay(speed)
                } else {
                    cmd.delay.unwrap_or(config.delay)
                };
                let jitter_val = cmd.jitter.unwrap_or(config.jitter);
                let pause_val = cmd.pause.unwrap_or(config.pause);

                if config.highlight {
                    let tokens = highlight_command(&resolved_text);
                    type_text_highlighted(&tokens, base_delay, jitter_val, pause_val);
                } else {
                    type_text(&resolved_text, base_delay, jitter_val, pause_val);
                }

                // Wait for input or auto-advance
                let nav = if let Some(ms) = cmd.wait.or(config.auto_advance) {
                    thread::sleep(Duration::from_millis(ms));
                    println!();
                    NavAction::Continue
                } else {
                    let action = wait_for_input(has_chapters);
                    // Handle navigation
                    match &action {
                        NavAction::NextChapter => {
                            if let Some(next) = demo
                                .chapters
                                .iter()
                                .find(|c| c.start > idx)
                            {
                                idx = next.start;
                                // Recalculate chapter_idx
                                chapter_idx = demo
                                    .chapters
                                    .iter()
                                    .position(|c| c.start == idx)
                                    .unwrap_or(chapter_idx);
                                continue;
                            }
                        }
                        NavAction::PrevChapter => {
                            // Find the chapter that contains the current step
                            let current_chapter = demo
                                .chapters
                                .iter()
                                .rposition(|c| c.start <= idx);
                            if let Some(ci) = current_chapter {
                                if ci > 0 {
                                    idx = demo.chapters[ci - 1].start;
                                    chapter_idx = ci - 1;
                                    continue;
                                } else {
                                    idx = demo.chapters[0].start;
                                    chapter_idx = 0;
                                    continue;
                                }
                            }
                        }
                        NavAction::JumpChapter(target) => {
                            if *target < demo.chapters.len() {
                                idx = demo.chapters[*target].start;
                                chapter_idx = *target;
                                continue;
                            }
                        }
                        NavAction::Continue => {}
                    }
                    action
                };
                let _ = nav;

                // Execute the command
                if let Some(ref fake) = cmd.fake_output {
                    // Fake output mode
                    if let Some(output_speed) = cmd.output_speed {
                        let output_delay = speed_to_delay(output_speed);
                        type_text(fake, output_delay, jitter_val, 0);
                        println!();
                    } else {
                        print!("{}", fake);
                        io::stdout().flush().unwrap();
                    }

                    if cmd.execute {
                        // Also run the real command (output is hidden since fake is shown)
                        let _ = Command::new("sh")
                            .arg("-c")
                            .arg(&resolved_text)
                            .stdin(Stdio::null())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                } else if !cmd.execute {
                    // No execution, no fake output — just typed the command
                } else if let Some(ref pattern) = cmd.wait_for {
                    run_command_wait_for(&resolved_text, pattern, cmd.timeout);
                } else if let Some(ref interactions) = cmd.interact {
                    let interaction_refs: Vec<Interaction> = interactions
                        .iter()
                        .map(|i| Interaction {
                            expect: i.expect.clone(),
                            send: i.send.clone(),
                        })
                        .collect();
                    run_command_interact(&resolved_text, &interaction_refs);
                } else {
                    let capture_ref = cmd.capture.as_ref().map(|c| Capture {
                        name: c.name.clone(),
                        pattern: c.pattern.clone(),
                    });
                    let (captured, _code) =
                        run_command(&resolved_text, capture_ref.as_ref());
                    if let Some(value) = captured {
                        if let Some(ref cap) = cmd.capture {
                            vars.insert(cap.name.clone(), value);
                        }
                    }
                }
            }
        }

        idx += 1;
    }

    // Teardown
    if let Some(ref teardown) = config.teardown {
        run_hidden_commands(teardown);
    }
}

// ---------------------------------------------------------------------------
// File watching
// ---------------------------------------------------------------------------

fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn wait_for_file_change(path: &Path) {
    let initial = file_mtime(path);
    loop {
        thread::sleep(Duration::from_millis(500));
        let current = file_mtime(path);
        if current != initial {
            return;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
        assert!(!config.highlight);
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
                assert!(cmd.execute);
                assert!(cmd.fake_output.is_none());
                assert!(cmd.wait_for.is_none());
                assert!(cmd.interact.is_none());
                assert!(cmd.if_condition.is_none());
                assert!(cmd.unless.is_none());
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
    fn test_config_no_steps_or_chapters_parses() {
        let yaml = "delay: 50\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.steps.is_empty());
        assert!(config.chapters.is_empty());
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

    // --- New feature tests ---

    #[test]
    fn test_comment_step() {
        let yaml = r#"
steps:
  - comment: "This is a narration"
    style: dim
  - text: "echo hello"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.steps.len(), 2);
        match &config.steps[0] {
            Step::Comment(c) => {
                assert_eq!(c.comment, "This is a narration");
                assert_eq!(c.style.as_deref(), Some("dim"));
            }
            _ => panic!("expected Comment step"),
        }
    }

    #[test]
    fn test_comment_step_no_style() {
        let yaml = r#"
steps:
  - comment: "Just a note"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        match &config.steps[0] {
            Step::Comment(c) => {
                assert_eq!(c.comment, "Just a note");
                assert!(c.style.is_none());
            }
            _ => panic!("expected Comment step"),
        }
    }

    #[test]
    fn test_fake_output() {
        let yaml = r#"
steps:
  - text: "curl https://api.example.com/health"
    fake_output: '{"status": "ok"}'
    execute: false
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        match &config.steps[0] {
            Step::Command(cmd) => {
                assert_eq!(cmd.fake_output.as_deref(), Some("{\"status\": \"ok\"}"));
                assert!(!cmd.execute);
            }
            _ => panic!("expected Command step"),
        }
    }

    #[test]
    fn test_auto_advance() {
        let yaml = r#"
auto_advance: 1500
steps:
  - text: "echo hello"
    wait: 2000
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.auto_advance, Some(1500));
        match &config.steps[0] {
            Step::Command(cmd) => assert_eq!(cmd.wait, Some(2000)),
            _ => panic!("expected Command step"),
        }
    }

    #[test]
    fn test_wait_for_pattern() {
        let yaml = r#"
steps:
  - text: "docker-compose logs -f"
    wait_for: "Listening on port 8080"
    timeout: 60
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        match &config.steps[0] {
            Step::Command(cmd) => {
                assert_eq!(cmd.wait_for.as_deref(), Some("Listening on port 8080"));
                assert_eq!(cmd.timeout, 60);
            }
            _ => panic!("expected Command step"),
        }
    }

    #[test]
    fn test_setup_teardown() {
        let yaml = r#"
setup:
  - "mkdir -p /tmp/test"
  - "echo setup"
teardown:
  - "rm -rf /tmp/test"
steps:
  - text: "ls /tmp/test"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.setup.as_ref().unwrap().len(), 2);
        assert_eq!(config.teardown.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_chapters() {
        let yaml = r#"
chapters:
  - name: "Setup"
    steps:
      - text: "git clone repo"
  - name: "Build"
    steps:
      - text: "cargo build"
      - text: "cargo test"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.chapters.len(), 2);
        assert_eq!(config.chapters[0].name, "Setup");
        assert_eq!(config.chapters[0].steps.len(), 1);
        assert_eq!(config.chapters[1].name, "Build");
        assert_eq!(config.chapters[1].steps.len(), 2);
    }

    #[test]
    fn test_resolve_demo_chapters() {
        let yaml = r#"
chapters:
  - name: "A"
    steps:
      - text: "cmd1"
      - text: "cmd2"
  - name: "B"
    steps:
      - text: "cmd3"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let demo = resolve_demo(&config);
        assert_eq!(demo.steps.len(), 3);
        assert_eq!(demo.chapters.len(), 2);
        assert_eq!(demo.chapters[0].name, "A");
        assert_eq!(demo.chapters[0].start, 0);
        assert_eq!(demo.chapters[1].name, "B");
        assert_eq!(demo.chapters[1].start, 2);
    }

    #[test]
    fn test_interact_deserialize() {
        let yaml = r#"
steps:
  - text: "npm init"
    interact:
      - expect: "package name:"
        send: "my-app"
      - expect: "version:"
        send: "1.0.0"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        match &config.steps[0] {
            Step::Command(cmd) => {
                let interactions = cmd.interact.as_ref().unwrap();
                assert_eq!(interactions.len(), 2);
                assert_eq!(interactions[0].expect, "package name:");
                assert_eq!(interactions[0].send, "my-app");
                assert_eq!(interactions[1].expect, "version:");
                assert_eq!(interactions[1].send, "1.0.0");
            }
            _ => panic!("expected Command step"),
        }
    }

    #[test]
    fn test_conditional_if() {
        let yaml = r#"
steps:
  - text: "docker build ."
    if: has_docker
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        match &config.steps[0] {
            Step::Command(cmd) => {
                assert_eq!(cmd.if_condition.as_deref(), Some("has_docker"));
            }
            _ => panic!("expected Command step"),
        }
    }

    #[test]
    fn test_conditional_unless() {
        let yaml = r#"
steps:
  - text: "podman build ."
    unless: has_docker
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        match &config.steps[0] {
            Step::Command(cmd) => {
                assert_eq!(cmd.unless.as_deref(), Some("has_docker"));
            }
            _ => panic!("expected Command step"),
        }
    }

    #[test]
    fn test_should_run_step_if_present() {
        let cmd = CommandStep {
            text: "test".to_string(),
            speed: None,
            delay: None,
            jitter: None,
            pause: None,
            capture: None,
            fake_output: None,
            output_speed: None,
            execute: true,
            wait_for: None,
            timeout: 30,
            wait: None,
            interact: None,
            if_condition: Some("myvar".to_string()),
            unless: None,
        };

        let mut vars = HashMap::new();
        assert!(!should_run_step(&cmd, &vars));

        vars.insert("myvar".to_string(), "value".to_string());
        assert!(should_run_step(&cmd, &vars));

        vars.insert("myvar".to_string(), "  ".to_string());
        assert!(!should_run_step(&cmd, &vars));
    }

    #[test]
    fn test_should_run_step_unless_present() {
        let cmd = CommandStep {
            text: "test".to_string(),
            speed: None,
            delay: None,
            jitter: None,
            pause: None,
            capture: None,
            fake_output: None,
            output_speed: None,
            execute: true,
            wait_for: None,
            timeout: 30,
            wait: None,
            interact: None,
            if_condition: None,
            unless: Some("myvar".to_string()),
        };

        let vars = HashMap::new();
        assert!(should_run_step(&cmd, &vars));

        let mut vars = HashMap::new();
        vars.insert("myvar".to_string(), "value".to_string());
        assert!(!should_run_step(&cmd, &vars));
    }

    #[test]
    fn test_highlight_command_basic() {
        let tokens = highlight_command("echo hello");
        assert!(!tokens.is_empty());
        // First word should be bold white
        assert_eq!(tokens[0], ('e', HL_BOLD_WHITE));
        assert_eq!(tokens[1], ('c', HL_BOLD_WHITE));
        assert_eq!(tokens[2], ('h', HL_BOLD_WHITE));
        assert_eq!(tokens[3], ('o', HL_BOLD_WHITE));
    }

    #[test]
    fn test_highlight_command_flags() {
        let tokens = highlight_command("ls --all -l");
        // Find the -- flags
        let flag_chars: Vec<_> = tokens
            .iter()
            .filter(|(_, color)| *color == HL_YELLOW)
            .collect();
        assert!(flag_chars.len() >= 6); // --all + -l
    }

    #[test]
    fn test_highlight_command_strings() {
        let tokens = highlight_command("echo \"hello world\"");
        let string_chars: Vec<_> = tokens
            .iter()
            .filter(|(_, color)| *color == HL_GREEN)
            .collect();
        assert!(string_chars.len() >= 13); // "hello world"
    }

    #[test]
    fn test_highlight_command_pipe() {
        let tokens = highlight_command("cat file | grep pattern");
        let pipe_chars: Vec<_> = tokens
            .iter()
            .filter(|(ch, color)| *ch == '|' && *color == HL_CYAN)
            .collect();
        assert_eq!(pipe_chars.len(), 1);
    }

    #[test]
    fn test_highlight_command_variable() {
        let tokens = highlight_command("echo $HOME");
        let var_chars: Vec<_> = tokens
            .iter()
            .filter(|(_, color)| *color == HL_MAGENTA)
            .collect();
        assert!(var_chars.len() >= 5); // $HOME
    }

    #[test]
    fn test_highlight_config() {
        let yaml = r#"
highlight: true
steps:
  - text: "echo hello"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.highlight);
    }

    #[test]
    fn test_style_to_ansi() {
        assert_eq!(style_to_ansi(Some("dim")), "\x1B[2m");
        assert_eq!(style_to_ansi(Some("bold")), "\x1B[1m");
        assert_eq!(style_to_ansi(Some("italic")), "\x1B[3m");
        assert_eq!(style_to_ansi(Some("red")), "\x1B[31m");
        assert_eq!(style_to_ansi(None), "\x1B[2m");
    }

    #[test]
    fn test_output_speed() {
        let yaml = r#"
steps:
  - text: "curl example.com"
    fake_output: "response body"
    output_speed: 40
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        match &config.steps[0] {
            Step::Command(cmd) => {
                assert_eq!(cmd.output_speed, Some(40));
            }
            _ => panic!("expected Command step"),
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    if cli.watch {
        loop {
            let config = load_config(&cli.config);
            run_demo(&config, &cli);
            eprintln!(
                "\n\x1B[2m[demonator] watching {} for changes... (Ctrl+C to exit)\x1B[0m",
                cli.config.display()
            );
            wait_for_file_change(&cli.config);
            // Clear screen before re-running
            print!("\x1B[2J\x1B[H");
            io::stdout().flush().unwrap();
        }
    } else {
        let config = load_config(&cli.config);
        run_demo(&config, &cli);
    }
}
