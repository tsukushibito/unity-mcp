//! MCP Server for mcp project with stdio transport

use clap::Parser;
use mcpr::{
    error::MCPError,
    schema::common::{Tool, ToolInputSchema},
    transport::{
        stdio::StdioTransport,
        Transport,
    },
};
use serde_json::Value;
use std::error::Error;
use std::collections::HashMap;
use log::{info, error, debug, warn};

/// CLI arguments
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug output
    #[arg(short, long)]
    debug: bool,
}

/// Server configuration
struct ServerConfig {
    /// Server name
    name: String,
    /// Server version
    version: String,
    /// Available tools
    tools: Vec<Tool>,
}

impl ServerConfig {
    /// Create a new server configuration
    fn new() -> Self {
        Self {
            name: "MCP Server".to_string(),
            version: "1.0.0".to_string(),
            tools: Vec::new(),
        }
    }

    /// Set the server name
    fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set the server version
    fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    /// Add a tool to the server
    fn with_tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }
}

/// Tool handler function type
type ToolHandler = Box<dyn Fn(Value) -> Result<Value, MCPError> + Send + Sync>;

/// High-level MCP server
struct Server<T> {
    config: ServerConfig,
    tool_handlers: HashMap<String, ToolHandler>,
    transport: Option<T>,
}

impl<T> Server<T> 
where 
    T: Transport
{
    /// Create a new MCP server with the given configuration
    fn new(config: ServerConfig) -> Self {
        Self {
            config,
            tool_handlers: HashMap::new(),
            transport: None,
        }
    }

    /// Register a tool handler
    fn register_tool_handler<F>(&mut self, tool_name: &str, handler: F) -> Result<(), MCPError>
    where
        F: Fn(Value) -> Result<Value, MCPError> + Send + Sync + 'static,
    {
        // Check if the tool exists in the configuration
        if !self.config.tools.iter().any(|t| t.name == tool_name) {
            return Err(MCPError::Protocol(format!(
                "Tool '{}' not found in server configuration",
                tool_name
            )));
        }

        // Register the handler
        self.tool_handlers
            .insert(tool_name.to_string(), Box::new(handler));

        info!("Registered handler for tool '{}'", tool_name);
        Ok(())
    }

    /// Start the server with the given transport
    fn start(&mut self, mut transport: T) -> Result<(), MCPError> {
        // Start the transport
        info!("Starting transport...");
        transport.start()?;

        // Store the transport
        self.transport = Some(transport);

        // Process messages
        info!("Processing messages...");
        self.process_messages()
    }

    /// Process incoming messages
    fn process_messages(&mut self) -> Result<(), MCPError> {
        info!("Server is running and waiting for client connections...");
        
        loop {
            let message = {
                let transport = self
                    .transport
                    .as_mut()
                    .ok_or_else(|| MCPError::Protocol("Transport not initialized".to_string()))?;

                // Receive a message
                match transport.receive() {
                    Ok(msg) => msg,
                    Err(e) => {
                        // For transport errors, log them but continue waiting
                        // This allows the server to keep running even if there are temporary connection issues
                        error!("Transport error: {}", e);
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                        continue;
                    }
                }
            };

            // Handle the message
            match message {
                mcpr::schema::json_rpc::JSONRPCMessage::Request(request) => {
                    let id = request.id.clone();
                    let method = request.method.clone();
                    let params = request.params.clone();

                    match method.as_str() {
                        "initialize" => {
                            info!("Received initialization request");
                            self.handle_initialize(id, params)?;
                        }
                        "tool_call" => {
                            info!("Received tool call request");
                            self.handle_tool_call(id, params)?;
                        }
                        "shutdown" => {
                            info!("Received shutdown request");
                            self.handle_shutdown(id)?;
                            break;
                        }
                        _ => {
                            warn!("Unknown method: {}", method);
                            self.send_error(
                                id,
                                -32601,
                                format!("Method not found: {}", method),
                                None,
                            )?;
                        }
                    }
                }
                _ => {
                    warn!("Unexpected message type");
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Handle initialization request
    fn handle_initialize(&mut self, id: mcpr::schema::json_rpc::RequestId, _params: Option<Value>) -> Result<(), MCPError> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| MCPError::Protocol("Transport not initialized".to_string()))?;

        // Create initialization response
        let response = mcpr::schema::json_rpc::JSONRPCResponse::new(
            id,
            serde_json::json!({
                "protocol_version": mcpr::constants::LATEST_PROTOCOL_VERSION,
                "server_info": {
                    "name": self.config.name,
                    "version": self.config.version
                },
                "tools": self.config.tools
            }),
        );

        // Send the response
        debug!("Sending initialization response");
        transport.send(&mcpr::schema::json_rpc::JSONRPCMessage::Response(response))?;

        Ok(())
    }

    /// Handle tool call request
    fn handle_tool_call(&mut self, id: mcpr::schema::json_rpc::RequestId, params: Option<Value>) -> Result<(), MCPError> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| MCPError::Protocol("Transport not initialized".to_string()))?;

        // Extract tool name and parameters
        let params = params.ok_or_else(|| {
            MCPError::Protocol("Missing parameters in tool call request".to_string())
        })?;

        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Protocol("Missing tool name in parameters".to_string()))?;

        let tool_params = params.get("parameters").cloned().unwrap_or(Value::Null);
        debug!("Tool call: {} with parameters: {:?}", tool_name, tool_params);

        // Find the tool handler
        let handler = self.tool_handlers.get(tool_name).ok_or_else(|| {
            MCPError::Protocol(format!("No handler registered for tool '{}'", tool_name))
        })?;

        // Call the handler
        match handler(tool_params) {
            Ok(result) => {
                // Create tool result response
                let response = mcpr::schema::json_rpc::JSONRPCResponse::new(
                    id,
                    serde_json::json!({
                        "result": result
                    }),
                );

                // Send the response
                debug!("Sending tool call response: {:?}", result);
                transport.send(&mcpr::schema::json_rpc::JSONRPCMessage::Response(response))?;
            }
            Err(e) => {
                // Send error response
                error!("Tool execution failed: {}", e);
                self.send_error(id, -32000, format!("Tool execution failed: {}", e), None)?;
            }
        }

        Ok(())
    }

    /// Handle shutdown request
    fn handle_shutdown(&mut self, id: mcpr::schema::json_rpc::RequestId) -> Result<(), MCPError> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| MCPError::Protocol("Transport not initialized".to_string()))?;

        // Create shutdown response
        let response = mcpr::schema::json_rpc::JSONRPCResponse::new(id, serde_json::json!({}));

        // Send the response
        debug!("Sending shutdown response");
        transport.send(&mcpr::schema::json_rpc::JSONRPCMessage::Response(response))?;

        // Close the transport
        info!("Closing transport");
        transport.close()?;

        Ok(())
    }

    /// Send an error response
    fn send_error(
        &mut self,
        id: mcpr::schema::json_rpc::RequestId,
        code: i32,
        message: String,
        data: Option<Value>,
    ) -> Result<(), MCPError> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| MCPError::Protocol("Transport not initialized".to_string()))?;

        // Create error response
        let error = mcpr::schema::json_rpc::JSONRPCMessage::Error(
            mcpr::schema::json_rpc::JSONRPCError::new(id, code, message.clone(), data),
        );

        // Send the error
        warn!("Sending error response: {}", message);
        transport.send(&error)?;

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Set log level based on debug flag
    if args.debug {
        log::set_max_level(log::LevelFilter::Debug);
        debug!("Debug logging enabled");
    }
    
    // Configure the server
    let server_config = ServerConfig::new()
        .with_name("mcp-server")
        .with_version("1.0.0")
        .with_tool(Tool {
            name: "hello".to_string(),
            description: Some("A simple hello world tool".to_string()),
            input_schema: ToolInputSchema {
                r#type: "object".to_string(),
                properties: Some([
                    ("name".to_string(), serde_json::json!({
                        "type": "string",
                        "description": "Name to greet"
                    }))
                ].into_iter().collect()),
                required: Some(vec!["name".to_string()]),
            },
        });
    
    // Create the server
    let mut server = Server::new(server_config);
    
    // Register tool handlers
    server.register_tool_handler("hello", |params: Value| {
        // Parse parameters
        let name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MCPError::Protocol("Missing name parameter".to_string()))?;
        
        info!("Handling hello tool call for name: {}", name);
        
        // Generate response
        let response = serde_json::json!({
            "message": format!("Hello, {}!", name)
        });
        
        Ok(response)
    })?;
    
    // Create transport and start the server
    info!("Starting stdio server");
    let transport = StdioTransport::new();
    
    info!("Starting mcp-server...");
    server.start(transport)?;
    
    Ok(())
}