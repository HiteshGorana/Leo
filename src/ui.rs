use colored::*;
use terminal_size::{Width, Height, terminal_size};

const RETRO_LION_OPEN: &str = r#"
     ━━━━━━━
    ┃  ● ●  ┃
    ┃   ^   ┃
    ┗━━━━━━━┛
     ┛     ┗
"#;

const RETRO_LION_BLINK: &str = r#"
     ━━━━━━━
    ┃  - -  ┃
    ┃   ^   ┃
    ┗━━━━━━━┛
     ┛     ┗
"#;

pub fn print_leo_header(model: &str, provider: &str) {
    let (width, _) = terminal_size().unwrap_or((Width(80), Height(24)));
    let width = width.0 as usize;

    let line = "━".repeat(width);
    println!("{}", line.yellow().dimmed());

    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    
    // Header line
    println!("  {} {}  {}", "Leo".yellow().bold(), version.black().bold(), "•".black().bold());

    // Animated sequence
    let frames = vec![RETRO_LION_OPEN, RETRO_LION_BLINK, RETRO_LION_OPEN];
    
    // Clear and print frames
    for (i, frame) in frames.iter().enumerate() {
        let logo_lines: Vec<&str> = frame.trim_matches('\n').lines().collect();
        
        let mut info_lines = Vec::new();
        info_lines.push(format!("Welcome back, {}!", whoami::realname().cyan().bold()));
        info_lines.push("".to_string());
        info_lines.push(format!("{}  •  {}", model.yellow(), provider.black().bold()));
        
        if let Ok(path) = std::env::current_dir() {
            info_lines.push(path.to_string_lossy().to_string().black().bold().to_string());
        }

        // Only print info lines on the last frame
        let is_last = i == frames.len() - 1;

        for (j, logo_line) in logo_lines.iter().enumerate() {
            let info = if is_last { info_lines.get(j).cloned().unwrap_or_default() } else { "".to_string() };
            println!("  {}    {}", logo_line.yellow(), info);
        }

        if !is_last {
            // Move cursor back up (5 lines for the logo)
            print!("\x1b[5A\r");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
    }

    println!("{}", line.yellow().dimmed());
}

pub fn print_api_call(model: &str) {
    println!("  {} {} {}", "✦".yellow(), "Gemini".black().bold(), model.cyan());
}

pub fn print_api_response() {
    // Minimalistic response indicator (just a subtle dot or similar)
    // Actually, maybe nothing at all if it's super fast, or just a small check
}

pub fn print_step(msg: &str) {
    println!("  {} {}", "•".green(), msg);
}

pub fn print_success(msg: &str) {
    println!("  {} {}", "✓".green().bold(), msg.green());
}

pub fn print_warning(msg: &str) {
    println!("  {} {}", "⚠️ ".yellow().bold(), msg.yellow());
}

pub fn print_error(msg: &str) {
    println!("  {} {}", "❌".red().bold(), msg.red());
}

pub fn print_thinking(msg: &str) {
    println!("  {} {}...", "∴".magenta(), msg);
}
