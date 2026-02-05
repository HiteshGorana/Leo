use colored::*;
use terminal_size::{Width, Height, terminal_size};

pub fn print_leo_header(model: &str, provider: &str) {
    let (width, _) = terminal_size().unwrap_or((Width(80), Height(24)));
    let width = width.0 as usize;

    let line = "─".repeat(width);
    println!("{}", line.black().bold());

    // Logo + Name
    let logo = "🦁";
    let name = "Leo".yellow().bold();
    let version = format!("v{}", env!("CARGO_PKG_VERSION")).black().bold();
    
    println!("  {} {} {}", logo, name, version);
    
    // Model + Provider Info
    let info = format!("  {}  •  {}", model, provider).cyan();
    println!("{}", info);

    // Path
    if let Ok(path) = std::env::current_dir() {
        let path_str = path.to_string_lossy().black().bold();
        println!("  {}", path_str);
    }

    println!("{}", line.black().bold());
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
