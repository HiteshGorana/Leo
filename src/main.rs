//! Leo CLI entry point

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;
use anyhow::Result;

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
            println!("\n👋 Bye!");
            std::process::exit(0);
        } else {
            println!("\n⚠️  Press Ctrl+C again to exit");
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
            println!("🦁 Initializing Leo...");
            leo::config::onboard()?;
            println!("✓ Leo is ready!");
            println!("\nNext steps:");
            println!("  1. Add your Gemini API key to ~/.leo/config.json");
            println!("     OR set \"provider\": \"google-cli\" and run 'leo login'");
            println!("  2. Chat: leo agent -m \"Hello!\"");
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
                println!("\n🐈 {}", response);
            } else {
                // Interactive mode
                println!("🦁 Interactive mode (Ctrl+C to exit)\n");
                run_agent_interactive(&config, &session).await?;
            }
        }
        
        Commands::Gateway { port, verbose } => {
            if verbose {
                tracing::info!("Starting gateway on port {}", port);
            }
            println!("🦁 Starting Leo gateway on port {}...", port);
            run_gateway(port).await?;
        }
        
        Commands::Status => {
            let config = leo::config::load()?;
            println!("🦁 Leo Status\n");
            println!("Workspace: {:?}", config.workspace);
            println!("Model: {}", config.model);
            println!("Provider: {}", config.provider);
            
            match config.provider.as_str() {
                "gemini" => {
                    println!("Gemini API: {}", if config.gemini_api_key.is_empty() { "not set" } else { "✓" });
                }
                "google-cli" => {
                    let has_creds = leo::auth::GeminiAuthProvider::has_valid_credentials()
                        .unwrap_or(false);
                    println!("OAuth credentials: {}", if has_creds { "✓" } else { "not set (run 'leo login')" });
                }
                _ => {
                    println!("Unknown provider: {}", config.provider);
                }
            }
        }
    }
    
    Ok(())
}

async fn run_agent_once(config: &leo::config::Config, message: &str, _session: &str) -> Result<String> {
    use leo::agent::{AgentLoop, Message, Context};
    use leo::agent::gemini::GeminiClient;
    use leo::agent::GeminiOAuthClient;
    
    let mut ctx = Context::new(config)?;
    
    let response = match config.provider.as_str() {
        "google-cli" => {
            // Use OAuth authentication
            let client = GeminiOAuthClient::from_cli(&config.model)?;
            let agent = AgentLoop::new(client, config.max_iterations);
            let msg = Message::user(message);
            agent.run(msg, &mut ctx).await?
        }
        _ => {
            // Default: Use API key authentication
            let client = GeminiClient::new(&config.gemini_api_key, &config.model);
            let agent = AgentLoop::new(client, config.max_iterations);
            let msg = Message::user(message);
            agent.run(msg, &mut ctx).await?
        }
    };
    
    Ok(response.content)
}

async fn run_agent_interactive(config: &leo::config::Config, session: &str) -> Result<()> {
    use std::io::{self, Write};
    
    loop {
        // Blue "You"
        print!("\x1b[1;34mYou\x1b[0m: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("👋 Bye!");
            break;
        }
        
        if input.is_empty() {
            continue;
        }
        
        // Green "Bot", Red "Error"
        match run_agent_once(config, input, session).await {
            Ok(response) => println!("\n\x1b[1;32mBot\x1b[0m: {}\n", response),
            Err(e) => println!("\n\x1b[1;31mError\x1b[0m: {}\n", e),
        }
    }
    
    Ok(())
}

async fn run_login(dry_run: bool) -> Result<()> {
    use leo::auth::{extract_cli_credentials, GeminiAuthProvider};
    
    println!("🔐 Extracting credentials from Gemini CLI...\n");
    
    match extract_cli_credentials() {
        Ok(creds) => {
            let masked_id = if creds.client_id.len() > 20 {
                format!("{}...{}", &creds.client_id[..10], &creds.client_id[creds.client_id.len()-10..])
            } else {
                creds.client_id.clone()
            };
            println!("✓ Found client_id: {}", masked_id);
            
            if dry_run {
                println!("\n(dry run - skipping OAuth flow)");
                return Ok(());
            }
            
            // Start OAuth flow
            let provider = GeminiAuthProvider::new(creds.client_id, creds.client_secret);
            let _token = provider.get_valid_token().await?;
            
            println!("\n✓ Authentication successful!");
            println!("  Credentials saved to ~/.leo/credentials.json");
            println!("\nYou can now use: leo agent -m \"Hello!\"");
        }
        Err(e) => {
            println!("❌ Failed to extract credentials: {}", e);
            println!("\nMake sure you have the Gemini CLI installed:");
            println!("  npm install -g @google/gemini-cli");
            return Err(e.into());
        }
    }
    
    Ok(())
}

async fn run_gateway(port: u16) -> Result<()> {
    use leo::agent::{AgentLoop, Context};
    use leo::agent::gemini::GeminiClient;
    use leo::agent::GeminiOAuthClient;
    use leo::adapters::{Channel, telegram::TelegramChannel};
    use leo::config::Config;

    println!("🐈 Loading configuration...");
    let config = leo::config::load()?;
    
    if !config.telegram.enabled {
        println!("⚠️ Telegram is disabled in config. Enable it and set 'token' to run the gateway.");
        return Ok(());
    }
    
    // Create Agent Loop with appropriate client
    // Note: We need a cloneable client for the adapter. 
    // GeminiClient and GeminiOAuthClient need to derive Clone or be wrapped in Arc in their impls.
    // Let's assume we can clone them for now (reqwest::Client is cheap to clone).
    
    println!("🐈 Initializing agent with provider: {}", config.provider);
    
    match config.provider.as_str() {
        "google-cli" => {
            let client = GeminiOAuthClient::from_cli(&config.model)?;
            let agent = AgentLoop::new(client, config.max_iterations);
            let channel = TelegramChannel::new(config, agent);
            println!("✓ Gateway started. Listening for Telegram messages...");
            channel.start().await?;
        }
        _ => {
            let client = GeminiClient::new(&config.gemini_api_key, &config.model);
            let agent = AgentLoop::new(client, config.max_iterations);
            let channel = TelegramChannel::new(config, agent);
            println!("✓ Gateway started. Listening for Telegram messages...");
            channel.start().await?;
        }
    };

    Ok(())
}


