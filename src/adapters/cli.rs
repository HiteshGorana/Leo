//! CLI adapter — interactive and single-message command line interface.
//!
//! Provides a clean channel abstraction for CLI interaction, extracted
//! from the main binary for better separation of concerns.

use std::io::{self, BufRead, Write};

use crate::agent::{AgentLoop, Context, LlmClient, Message, Response};
use crate::Result;

/// CLI channel for interactive agent sessions.
pub struct CliChannel<C: LlmClient> {
    agent: AgentLoop<C>,
    context: Context,
    history: Vec<Message>,
}

impl<C: LlmClient> CliChannel<C> {
    /// Create a new CLI channel.
    pub fn new(agent: AgentLoop<C>, context: Context) -> Self {
        Self {
            agent,
            context,
            history: Vec::new(),
        }
    }

    /// Run a single message and return the response.
    pub async fn run_once(&mut self, message: &str) -> Result<Response> {
        let msg = Message::user(message);
        let response = self.agent.run(&self.history, msg.clone(), &mut self.context).await?;
        
        // Update history
        self.history.push(msg);
        self.history.push(Message::assistant(response.content.clone()));
        
        Ok(response)
    }

    /// Run interactive REPL loop.
    pub async fn run_interactive(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            // Print prompt
            print!("\n> ");
            stdout.flush()?;

            // Read input
            let mut line = String::new();
            if stdin.lock().read_line(&mut line)? == 0 {
                // EOF
                break;
            }

            let input = line.trim();
            if input.is_empty() {
                continue;
            }

            // Check for exit commands
            if matches!(input.to_lowercase().as_str(), "exit" | "quit" | "q") {
                println!("Goodbye! 👋");
                break;
            }

            // Process message
            match self.run_once(input).await {
                Ok(response) => {
                    println!("\n{}", response.content);
                }
                Err(e) => {
                    eprintln!("\nError: {e}");
                }
            }
        }

        Ok(())
    }

    /// Clear conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Get current history length.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}
