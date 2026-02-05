use colored::*;
use terminal_size::{Width, Height, terminal_size};

#[derive(Debug, Clone, Copy)]
pub enum LionEmotion {
    Normal,
    Blink,
    Happy,
    Sad,
    Fear,
    Anger,
    Disgust,
    Surprise,
    Sleep,
    Jealousy,
    Empathy,
    Anxiety,
    Contempt,
    Loneliness,
    Boredom,
}

impl LionEmotion {
    pub fn get_face(&self) -> (&str, &str) {
        match self {
            LionEmotion::Normal => (" ● ● ", "  ^  "),
            LionEmotion::Blink => (" - - ", "  ^  "),
            LionEmotion::Happy => (" ^ ^ ", "  v  "),
            LionEmotion::Sad => (" T T ", "  -  "),
            LionEmotion::Fear => (" O O ", "  w  "),
            LionEmotion::Anger => (" \\ / ", "  ~  "),
            LionEmotion::Disgust => (" x o ", "  -  "),
            LionEmotion::Surprise => (" ! ! ", "  o  "),
            LionEmotion::Sleep => (" z Z ", "     "),
            LionEmotion::Jealousy => (" ¬ ¬ ", "  ^  "),
            LionEmotion::Empathy => (" u u ", "  v  "),
            LionEmotion::Anxiety => (" ; ; ", "  .  "),
            LionEmotion::Contempt => (" _ _ ", "  /  "),
            LionEmotion::Loneliness => (" · · ", "     "),
            LionEmotion::Boredom => (" - - ", "  ~  "),
        }
    }

    pub fn render(&self) -> String {
        let (eyes, mouth) = self.get_face();
        format!(r#"
     ━━━━━━━
    ┃{}┃
    ┃{}┃
    ┗━━━━━━━┛
     ┛     ┗
"#, eyes, mouth).trim_matches('\n').to_string()
    }
}

pub fn print_leo_header(model: &str, provider: &str) {
    print_leo_header_with_emotion(model, provider, LionEmotion::Normal);
}

pub fn print_leo_header_with_emotion(model: &str, provider: &str, emotion: LionEmotion) {
    let (width, _) = terminal_size().unwrap_or((Width(80), Height(24)));
    let width = width.0 as usize;

    let line = "━".repeat(width);
    println!("{}", line.yellow().dimmed());

    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    
    // Header line
    println!("  {} {}  {}", "Leo".yellow().bold(), version.black().bold(), "•".black().bold());

    // Animated sequence: Start -> Blink -> Target Emotion
    let frames = vec![LionEmotion::Normal, LionEmotion::Blink, emotion];
    
    // Clear and print frames
    for (i, frame) in frames.iter().enumerate() {
        let logo_rendered = frame.render();
        let logo_lines: Vec<&str> = logo_rendered.lines().collect();
        
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
}

pub fn print_leo_face(emotion: LionEmotion) {
    println!("{}\n", emotion.render().yellow());
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
