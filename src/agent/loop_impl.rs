//! Agent loop - core message processing

use tracing::{debug, info};
use crate::Result;
use crate::error::Error;
use super::llm::LlmClient;
use super::message::{Message, Response, ToolCallRequest};
use super::context::Context;

/// The agent loop processes messages through LLM and tool execution
pub struct AgentLoop<C: LlmClient> {
    client: C,
    max_iterations: usize,
}

impl<C: LlmClient> AgentLoop<C> {
    /// Create a new agent loop
    pub fn new(client: C, max_iterations: usize) -> Self {
        Self {
            client,
            max_iterations,
        }
    }
    
    /// Run the agent loop for a single message
    pub async fn run(&self, message: Message, ctx: &mut Context) -> Result<Response> {
        // Build messages from context
        let mut messages = ctx.build_messages(&[], &message.content);
        
        info!("Starting agent loop with message: {}", message.content);
        
        for iteration in 0..self.max_iterations {
            debug!("Iteration {}/{}", iteration + 1, self.max_iterations);
            
            // Get tool definitions
            let tools = ctx.tool_runner.definitions();
            
            // Call LLM
            let response = self.client.chat(&messages, &tools).await?;
            
            // Check if done
            if !response.has_tool_calls() {
                let content = response.content.unwrap_or_default();
                info!("Agent completed with response: {} chars", content.len());
                return Ok(Response::new(content));
            }
            
            // Add assistant message with tool calls
            messages.push(Message::assistant_with_tools(
                response.content.clone().unwrap_or_default(),
                response.tool_calls.clone(),
            ));
            
            // Execute tool calls
            for tool_call in &response.tool_calls {
                let result = self.execute_tool(ctx, tool_call).await;
                messages.push(Message::tool_result(&tool_call.id, result));
            }
        }
        
        Err(Error::MaxIterations)
    }
    
    async fn execute_tool(&self, ctx: &mut Context, tool_call: &ToolCallRequest) -> String {
        debug!("Executing tool: {} with args: {}", tool_call.name, tool_call.arguments);
        
        match ctx.tool_runner.execute(&tool_call.name, tool_call.arguments.clone()).await {
            Ok(result) => {
                debug!("Tool {} succeeded: {} chars", tool_call.name, result.len());
                result
            }
            Err(e) => {
                let error_msg = format!("Error: {}", e);
                debug!("Tool {} failed: {}", tool_call.name, error_msg);
                error_msg
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::llm::FakeLlmClient;
    
    #[tokio::test]
    async fn test_agent_loop_simple() {
        let client = FakeLlmClient::new(vec!["Hello, human!"]);
        let mut ctx = Context::test();
        let agent = AgentLoop::new(client, 10);
        
        let msg = Message::user("Hi there");
        let response = agent.run(msg, &mut ctx).await.unwrap();
        
        assert_eq!(response.content, "Hello, human!");
    }
    
    #[tokio::test]
    async fn test_agent_loop_with_tool() {
        use serde_json::json;
        
        let client = FakeLlmClient::with_tool_call(
            "read_file",
            json!({"path": "test.txt"}),
            "The file contains: test content"
        );
        let mut ctx = Context::test();
        let agent = AgentLoop::new(client, 10);
        
        let msg = Message::user("Read test.txt");
        let response = agent.run(msg, &mut ctx).await.unwrap();
        
        assert_eq!(response.content, "The file contains: test content");
    }
}
