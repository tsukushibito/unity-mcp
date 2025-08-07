use rmcp::{handler::server::tool::ToolRouter, tool_handler, tool_router, ServerHandler};

#[derive(Debug, Clone)]
pub struct Unity {
    tool_router: ToolRouter<Unity>,
}

#[tool_router]
impl Unity {
    pub(crate) fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_handler]
impl ServerHandler for Unity {}
