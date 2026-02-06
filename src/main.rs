//! Leo CLI entry point

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;
use anyhow::Result;
use colored::Colorize;

#[derive(Parser)]
#[command(name = "leo")]
#[command(about = "🦁 Leo - Ultra-lightweight personal AI assistant")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Leo configuration and workspace
    Onboard,
    
    /// Chat with the agent
    Agent {
        /// Message to send to the agent
        #[arg(short, long)]
        message: Option<String>,
        
        /// Session ID
        #[arg(short, long, default_value = "cli:default")]
        session: String,
    },
    
    /// Login to Google for OAuth authentication
    Login {
        /// Dry run - only show if credentials can be extracted
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Logout - remove stored OAuth credentials
    Logout,
    
    /// Start the Leo gateway
    Gateway {
        /// Gateway port
        #[arg(short, long, default_value_t = 18790)]
        port: u16,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Show Leo status
    Status,

    /// Reset Leo - delete all configuration and data
    Reset,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
        
    // Setup Global Ctrl+C handler
    let exit_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let r = exit_flag.clone();
    
    ctrlc::set_handler(move || {
        if r.load(std::sync::atomic::Ordering::SeqCst) {
            println!("\n  → Bye!");
            std::process::exit(0);
        } else {
            println!("\n  ! Press Ctrl+C again to exit");
            r.store(true, std::sync::atomic::Ordering::SeqCst);
            
            // Reset flag after 3 seconds
            let r2 = r.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));
                r2.store(false, std::sync::atomic::Ordering::SeqCst);
            });
        }
    }).ok();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Onboard => {
            leo::config::onboard()?;
            
            // Reload config to check provider
            let config = leo::config::load()?;
            if config.provider == "google-cli" {
                println!();
                run_login(false).await?;
            }
        }
        
        Commands::Login { dry_run } => {
            run_login(dry_run).await?;
        }
        
        Commands::Logout => {
            leo::auth::delete_credentials()?;
            println!("✓ Logged out successfully");
        }
        
        Commands::Agent { message, session } => {
            let config = leo::config::load()?;
            
            if let Some(msg) = message {
                // Single message mode
                let response = run_agent_once(&config, &msg, &session).await?;
                println!("\n  {} {}", "🦁".green(), response);
            } else {
                // Interactive mode
                leo::ui::print_leo_header_with_emotion(&config.model, &config.provider, leo::ui::LionEmotion::Happy);
                run_agent_interactive(&config, &session).await?;
            }
        }
        
        Commands::Gateway { port, verbose } => {
            let config = leo::config::load()?;
            leo::ui::print_leo_header_with_emotion(&config.model, &format!("Gateway:{}", config.provider), leo::ui::LionEmotion::Anxiety);
            
            if verbose {
                tracing::info!("Starting gateway on port {}", port);
            }
            run_gateway(port).await?;
        }
        
        Commands::Status => {
            let config = leo::config::load()?;
            leo::ui::print_leo_header_with_emotion(&config.model, &config.provider, leo::ui::LionEmotion::Normal);
            
            println!("  {} {:?}", "Workspace:".black().bold(), config.workspace);
            
            match config.provider.as_str() {
                "gemini" => {
                    let status = if config.gemini_api_key.is_empty() { 
                        "not set".red() 
                    } else { 
                        "✓".green() 
                    };
                    println!("  {} {}", "Gemini API:".black().bold(), status);
                }
                "google-cli" => {
                    let has_creds = leo::auth::GeminiAuthProvider::has_valid_credentials()
                        .unwrap_or(false);
                    let status = if has_creds { 
                        "✓".green() 
                    } else { 
                        "not set (run 'leo login')".red() 
                    };
                    println!("  {} {}", "OAuth credentials:".black().bold(), status);
                }
                _ => {
                    println!("  {} {}", "Unknown provider:".black().bold(), config.provider);
                }
            }
            println!();
        }
        
        Commands::Reset => {
            leo::ui::print_leo_header_with_emotion("Maintenance", "Local", leo::ui::LionEmotion::Fear);
            leo::config::reset()?;
        }
    }
    
    Ok(())
}

async fn run_agent_once(config: &leo::config::Config, message: &str, _session: &str) -> Result<String> {
    use leo::agent::{AgentLoop, Message, Context};
    use leo::agent::GeminiClient;
    use leo::agent::GeminiOAuthClient;
    
    let mut ctx = Context::new(config)?;
    
    let response = match config.provider.as_str() {
        "google-cli" => {
            // Use OAuth authentication
            let client = GeminiOAuthClient::from_cli(&config.model)?;
            let agent = AgentLoop::new(client, config.max_iterations);
            let msg = Message::user(message);
            agent.run(&[], msg, &mut ctx).await?
        }
        _ => {
            // Default: Use API key authentication
            let client = GeminiClient::new(&config.gemini_api_key, &config.model);
            let agent = AgentLoop::new(client, config.max_iterations);
            let msg = Message::user(message);
            agent.run(&[], msg, &mut ctx).await?
        }
    };
    
    Ok(response.content)
}

async fn run_agent_interactive(config: &leo::config::Config, _session: &str) -> Result<()> {
    use std::io::{self, Write};
    use leo::agent::{AgentLoop, Message, Context};
    use leo::agent::GeminiClient;
    use leo::agent::GeminiOAuthClient;
    use leo::ui;
    
    // Initialize Context ONCE to keep tools (like Browser Bridge) alive
    ui::print_thinking("Initializing tools");
    let mut ctx = Context::new(config)?;
    ui::print_success("Ready! (Browser Extension can now connect)\n");
    
    // History for interactive session
    let mut history: Vec<Message> = Vec::new();

    loop {
        // Blue "You"
        print!("  \x1b[1;34mYou\x1b[0m: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("\n  → Bye!");
            break;
        }
        
        if input.is_empty() {
            continue;
        }
        
        // Green "Bot", Red "Error"
        // We inline the agent run logic here to reuse ctx
        let history_clone = history.clone();
        let result = async {
            let response = match config.provider.as_str() {
                "google-cli" => {
                    let client = GeminiOAuthClient::from_cli(&config.model)?;
                    let agent = AgentLoop::new(client, config.max_iterations);
                    let msg = Message::user(input);
                    agent.run(&history_clone, msg, &mut ctx).await?
                }
                _ => {
                    let client = GeminiClient::new(&config.gemini_api_key, &config.model);
                    let agent = AgentLoop::new(client, config.max_iterations);
                    let msg = Message::user(input);
                    agent.run(&history_clone, msg, &mut ctx).await?
                }
            };
            Ok::<leo::agent::Response, anyhow::Error>(response)
        }.await;

        match result {
            Ok(response) => {
                let content = response.content;
                println!("\n  \x1b[1;32mLeo\x1b[0m: {}\n", content);
                
                // Update history
                history.push(Message::user(input));
                history.push(Message::assistant(content));
            },
            Err(e) => println!("\n  \x1b[1;31mError\x1b[0m: {}\n", e),
        }
    }
    Ok(())
}

async fn run_login(dry_run: bool) -> Result<()> {
    use leo::auth::{extract_cli_credentials, GeminiAuthProvider};
    use leo::ui;
    
    ui::print_leo_header_with_emotion("Authentication", "Google SDK", ui::LionEmotion::Happy);
    ui::print_thinking("Extracting credentials from Gemini CLI");
    
    match extract_cli_credentials() {
        Ok(creds) => {
            let masked_id = if creds.client_id.len() > 20 {
                format!("{}...{}", &creds.client_id[..10], &creds.client_id[creds.client_id.len()-10..])
            } else {
                creds.client_id.clone()
            };
            ui::print_step(&format!("Found client_id: {}", masked_id));
            
            if dry_run {
                println!("\n  (dry run - skipping OAuth flow)");
                return Ok(());
            }
            
            // Start OAuth flow
            let provider = GeminiAuthProvider::new(creds.client_id, creds.client_secret);
            let _token = provider.get_valid_token().await?;
            
            println!();
            ui::print_success("Authentication successful!");
            ui::print_step("Credentials saved to ~/.leo/credentials.json");
            ui::print_step("You can now use: leo agent -m \"Hello!\"");
        }
        Err(e) => {
            ui::print_error(&format!("Failed to extract credentials: {}", e));
            println!("\n  Make sure you have the Gemini CLI installed:");
            println!("    npm install -g @google/gemini-cli");
            return Err(e.into());
        }
    }
    
    Ok(())
}

async fn run_gateway(_port: u16) -> Result<()> {
    use leo::agent::AgentLoop;
    use leo::agent::GeminiClient;
    use leo::agent::GeminiOAuthClient;
    use leo::adapters::{Channel, telegram::TelegramChannel};

    println!("∴ Loading configuration...");
    let mut config = leo::config::load()?;
    
    if !config.telegram.enabled {
        use inquire::Select;
        println!("⚠️ No Gateway channels are enabled in your configuration.");
        
        let gateways = vec!["Telegram Bot", "WhatsApp (Coming soon)", "Slack (Coming soon)", "Skip"];
        let choice = Select::new("Which gateway would you like to setup?", gateways).prompt()
            .map_err(|e| anyhow::anyhow!("Prompt failed: {}", e))?;
            
        match choice {
            "Telegram Bot" => {
                leo::config::setup_telegram_gateway(&mut config)?;
                leo::config::save(&config)?;
            }
            "Skip" => {
                println!("  Gateway cannot start without an active channel.");
                return Ok(());
            }
            _ => {
                println!("  {} is not yet supported. Choose Telegram instead!", choice);
                return Ok(());
            }
        }
    }
    
    // Create Agent Loop with appropriate client
    // Note: We need a cloneable client for the adapter. 
    // GeminiClient and GeminiOAuthClient need to derive Clone or be wrapped in Arc in their impls.
    // Let's assume we can clone them for now (reqwest::Client is cheap to clone).
    
    println!("∴ Initializing agent with provider: {}", config.provider);
    
    // Initialize Context once to keep Browser Bridge alive
    println!("🦁 Initializing tools...");
    let ctx = leo::agent::Context::new(&config)?;
    println!("✓ Tools ready! (Browser Extension can now connect)");

    match config.provider.as_str() {
        "google-cli" => {
            let client = GeminiOAuthClient::from_cli(&config.model)?;
            let agent = AgentLoop::new(client, config.max_iterations);
            let channel = TelegramChannel::new(config, agent, ctx);
            println!("✓ Gateway started. Listening for Telegram messages...");
            channel.start().await?;
        }
        _ => {
            let client = GeminiClient::new(&config.gemini_api_key, &config.model);
            let agent = AgentLoop::new(client, config.max_iterations);
            let channel = TelegramChannel::new(config, agent, ctx);
            println!("✓ Gateway started. Listening for Telegram messages...");
            channel.start().await?;
        }
    };

    Ok(())
}


