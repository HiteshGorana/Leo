//! Telegram adapter using teloxide

use teloxide::prelude::*;
use teloxide::types::{MediaKind, MessageKind};
use crate::Result;
// use crate::error::Error;
use crate::config::Config;
use super::Channel;
use crate::agent::{AgentLoop, Context, Message, InboundMessage, LlmClient};
use tokio::sync::Mutex;
use std::sync::Arc;
use tracing::{info, error, debug};
use std::collections::HashMap;

/// Telegram channel adapter
pub struct TelegramChannel<C: LlmClient + 'static> {
    bot: Bot,
    config: Config,
    agent_loop: Arc<AgentLoop<C>>,
    // Simple in-memory session lock to prevent concurrent processing for same chat
    // In production this might need a distributed lock or queue
    locks: Arc<Mutex<HashMap<ChatId, Arc<Mutex<()>>>>>,
}

impl<C: LlmClient + Clone> TelegramChannel<C> {
    pub fn new(config: Config, agent_loop: AgentLoop<C>) -> Self {
        let bot = Bot::new(&config.telegram.token);
        Self {
            bot,
            config,
            agent_loop: Arc::new(agent_loop),
            locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn handle_message(&self, message: teloxide::types::Message) -> Result<()> {
        let chat_id = message.chat.id;
        let user = message.from();
        
        // Authorization check
        if !self.is_allowed(user) {
            debug!("Ignoring message from unauthorized user: {:?}", user);
            return Ok(());
        }

        // Get text content
        let text = match message.kind {
            MessageKind::Common(ref common) => match &common.media_kind {
                MediaKind::Text(media) => &media.text,
                _ => return Ok(()), // Ignore non-text messages for now
            },
            _ => return Ok(()),
        };

        info!("Received message from {}: {}", chat_id, text);

        // Send "typing" action
        let _ = self.bot.send_chat_action(chat_id, teloxide::types::ChatAction::Typing).await;

        // Session locking
        let lock = {
            let mut locks = self.locks.lock().await;
            locks.entry(chat_id).or_insert_with(|| Arc::new(Mutex::new(()))).clone()
        };
        let _guard = lock.lock().await;

        // Prepare context
        // In a real app we would load a persisted session ID. 
        // For now, we use the chat_id as the session key.
        // Also note: we're creating a fresh Context for each message here.
        // In the full Python version, SessionManager handles history.
        // TODO: Implement proper SessionManager in Rust.
        let mut ctx = Context::new(&self.config)?;
        
        // Convert to Agent Message
        let msg = Message::user(text);
        
        // Run Agent Loop
        match self.agent_loop.run(msg, &mut ctx).await {
            Ok(response) => {
                // Send response
                self.bot.send_message(chat_id, response.content).await?;
            }
            Err(e) => {
                error!("Agent loop processing error: {}", e);
                self.bot.send_message(chat_id, format!("❌ Error: {}", e)).await?;
            }
        }

        Ok(())
    }

    fn is_allowed(&self, user: Option<&teloxide::types::User>) -> bool {
        if self.config.telegram.allow_from.is_empty() {
            return true; // If allow list is empty, allow all (or maybe default deny? generic safety says deny)
            // But for a personal bot, usually we want to restrict.
            // Let's assume if list is empty, it's open (dev mode), or stricter:
            // return false; 
        }
        
        let Some(user) = user else { return false };
        let username = user.username.as_deref().unwrap_or("");
        let id = user.id.to_string();
        
        self.config.telegram.allow_from.iter().any(|allowed| {
            allowed == username || allowed == &id
        })
    }
}

// Helper to wrap the event loop
async fn run_telegram_loop<C: LlmClient + Clone + 'static>(channel: Arc<TelegramChannel<C>>) {
    let handler = Update::filter_message()
        .endpoint(move |_bot: Bot, msg: teloxide::types::Message, channel: Arc<TelegramChannel<C>>| async move {
            if let Err(e) = channel.handle_message(msg).await {
                error!("Error handling telegram message: {}", e);
            }
            respond(())
        });

    Dispatcher::builder(channel.bot.clone(), handler)
        .dependencies(dptree::deps![channel])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

impl<C: LlmClient + Clone + 'static> Channel for TelegramChannel<C> {
    fn name(&self) -> &str {
        "telegram"
    }

    fn start(&self) -> impl std::future::Future<Output = std::result::Result<(), crate::error::Error>> + Send {
        let this = Arc::new(Self {
            bot: self.bot.clone(),
            config: self.config.clone(),
            agent_loop: self.agent_loop.clone(),
            locks: self.locks.clone(),
        });
        
        async move {
            info!("Starting Telegram bot...");
            run_telegram_loop(this).await;
            Ok(())
        }
    }

    fn stop(&self) -> impl std::future::Future<Output = std::result::Result<(), crate::error::Error>> + Send {
        async {
            // Teloxide dispatcher handles Ctrl+C, so manual stop isn't strictly needed for simple use cases
            Ok(())
        }
    }
}
