use crate::error::Result;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use color_eyre::eyre::{Error, OptionExt};
use rmcp::{
    handler::server::tool::Parameters,
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router, ErrorData, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::oneshot::Receiver;
use tokio::sync::{mpsc, watch, Mutex};
use tokio::time::Duration;
use uuid::Uuid;
// use scraper::{Html, Selector};

pub const STUDIO_PLUGIN_PORT: u16 = 44755;
const LONG_POLL_DURATION: Duration = Duration::from_secs(15);

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ToolArguments {
    args: ToolArgumentValues,
    id: Option<Uuid>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RunCommandResponse {
    response: String,
    id: Uuid,
}

pub struct AppState {
    process_queue: VecDeque<ToolArguments>,
    output_map: HashMap<Uuid, mpsc::UnboundedSender<Result<String>>>,
    waiter: watch::Receiver<()>,
    trigger: watch::Sender<()>,
}
pub type PackedState = Arc<Mutex<AppState>>;

impl AppState {
    pub fn new() -> Self {
        let (trigger, waiter) = watch::channel(());
        Self {
            process_queue: VecDeque::new(),
            output_map: HashMap::new(),
            waiter,
            trigger,
        }
    }
}

impl ToolArguments {
    fn new(args: ToolArgumentValues) -> (Self, Uuid) {
        Self { args, id: None }.with_id()
    }
    fn with_id(self) -> (Self, Uuid) {
        let id = Uuid::new_v4();
        (
            Self {
                args: self.args,
                id: Some(id),
            },
            id,
        )
    }
}
#[derive(Clone)]
pub struct RBXStudioServer {
    state: PackedState,
    tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
}

#[tool_handler]
impl ServerHandler for RBXStudioServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "User run_command to query data from Roblox Studio place or to change it"
                    .to_string(),
            ),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct RunCommand {
    #[schemars(
        description = "Luau code/command to execute in Studio (e.g. `workspace.Part.Color = Color3.new(1,0,0)`)"
    )]
    command: String,
}
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct InsertModel {
    query: Option<String>,
    #[schemars(description = "Optional Asset ID to insert directly")]
    asset_id: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct SearchMarketplace {
    #[schemars(description = "Search query")]
    query: String,
    #[schemars(description = "Asset type (e.g. Model, Plugin, Audio). Default: Model")]
    asset_type: Option<String>,
    #[schemars(description = "Number of results. Default: 10")]
    limit: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct DownloadAsset {
    #[schemars(description = "Asset ID to download")]
    asset_id: u64,
    #[schemars(description = "Filename to save as (without extension). Defaults to asset_id")]
    file_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetInstanceProperties {
    #[schemars(
        description = "Roblox instance path using dot notation (e.g., \"game.Workspace.Part\")"
    )]
    instance_path: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct SetProperty {
    #[schemars(description = "Path to the instance (e.g., \"game.Workspace.Part\")")]
    instance_path: String,
    #[schemars(description = "Name of the property to set")]
    property_name: String,
    #[schemars(description = "Value to set the property to (any type)")]
    property_value: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct SearchWebScripts {
    #[schemars(description = "Query to search for")]
    query: String,
    #[schemars(description = "Search depth (basic or advanced). Default: basic")]
    depth: Option<String>,
    #[schemars(
        description = "Use Research Agent for deep analysis (slower but better). Default: false"
    )]
    use_research: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct TavilyExtract {
    #[schemars(description = "URLs to extract content from")]
    urls: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct FetchUrlContent {
    #[schemars(description = "URL to fetch raw content from")]
    url: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct InstallSystem {
    #[schemars(
        description = "Name of the system to install (e.g. 'Quest System', 'Fireball Skill')"
    )]
    system_name: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct SmartUnpack {
    #[schemars(description = "Name of the container Instance (Model/Folder) to unpack")]
    target_name: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct SearchCreatorStore {
    #[schemars(description = "Search query for assets")]
    query: String,
    #[schemars(
        description = "Asset type: Audio, Model, Decal, Plugin, MeshPart, Video, FontFamily. Default: Model"
    )]
    asset_type: Option<String>,
    #[schemars(description = "Maximum number of results. Default: 10, Max: 100")]
    limit: Option<u32>,
    #[schemars(description = "Download found assets to Desktop folder. Default: false")]
    download: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct InsertAssetsArgs {
    #[schemars(description = "List of asset IDs to insert into Roblox Studio")]
    asset_ids: Vec<u64>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct CreateScript {
    #[schemars(description = "Name of the script")]
    name: String,
    #[schemars(description = "Parent instance path")]
    parent: String,
    #[schemars(
        description = "Type of script: 'Script', 'LocalScript', or 'ModuleScript'. Default: Script"
    )]
    script_type: Option<String>,
    #[schemars(description = "Initial source code")]
    source: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct UpdateScript {
    #[schemars(description = "Path to the script instance")]
    instance_path: String,
    #[schemars(description = "New source code")]
    source: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct ReadScript {
    #[schemars(description = "Path to the script instance")]
    instance_path: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetFileTree {
    #[schemars(
        description = "Roblox instance path to start from using dot notation. Defaults to game root if empty."
    )]
    path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetServices {
    #[schemars(description = "Optional specific service name to query")]
    service_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetProjectStructure {
    #[schemars(description = "Optional path to start from")]
    path: Option<String>,
    #[schemars(description = "Maximum depth to traverse (default: 3)")]
    max_depth: Option<u32>,
    #[schemars(description = "Show only scripts and script containers")]
    scripts_only: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetInstanceChildren {
    #[schemars(description = "Roblox instance path")]
    instance_path: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct SearchFiles {
    #[schemars(description = "Search query")]
    query: String,
    #[schemars(description = "Type of search: \"name\", \"type\", or \"content\"")]
    search_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct SearchObjects {
    #[schemars(description = "Search query")]
    query: String,
    #[schemars(description = "Type of search: \"name\", \"class\", or \"property\"")]
    search_type: Option<String>,
    #[schemars(description = "Property name when searchType is \"property\"")]
    property_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct SearchByProperty {
    #[schemars(description = "Name of the property to search")]
    property_name: String,
    #[schemars(description = "Value to search for")]
    property_value: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct CreateObject {
    #[serde(rename = "className")]
    class_name: String,
    parent: String,
    name: Option<String>,
    properties: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct DeleteObject {
    instance_path: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct MassSetProperty {
    paths: Vec<String>,
    property_name: String,
    property_value: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct MassGetProperty {
    paths: Vec<String>,
    property_name: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct SetCalculatedProperty {
    paths: Vec<String>,
    #[schemars(description = "Property to set")]
    property_name: String,
    #[schemars(description = "Formula to calculate value (uses 'index', 'Position.X', etc)")]
    formula: String,
    #[schemars(description = "Optional variables for the formula")]
    variables: Option<std::collections::HashMap<String, f64>>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct SetRelativeProperty {
    paths: Vec<String>,
    property_name: String,
    #[schemars(description = "Operation: add, subtract, multiply, divide, power")]
    operation: String,
    value: serde_json::Value,
    #[schemars(description = "Optional component (X, Y, Z) for Vector3")]
    component: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetPlaceInfo {}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetClassInfo {
    #[serde(rename = "className")]
    class_name: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
enum ToolArgumentValues {
    RunCommand(RunCommand),
    InsertModel(InsertModel),
    GetInstanceProperties(GetInstanceProperties),
    SetProperty(SetProperty),
    GetFileTree(GetFileTree),
    GetServices(GetServices),
    GetProjectStructure(GetProjectStructure),
    GetInstanceChildren(GetInstanceChildren),
    SearchFiles(SearchFiles),
    SearchObjects(SearchObjects),
    SearchByProperty(SearchByProperty),
    CreateObject(CreateObject),
    DeleteObject(DeleteObject),
    MassSetProperty(MassSetProperty),
    MassGetProperty(MassGetProperty),
    SetCalculatedProperty(SetCalculatedProperty),
    SetRelativeProperty(SetRelativeProperty),
    GetPlaceInfo(GetPlaceInfo),
    GetClassInfo(GetClassInfo),
    SearchMarketplace(SearchMarketplace),
    DownloadAsset(DownloadAsset),
    SearchWebScripts(SearchWebScripts),
    TavilyExtract(TavilyExtract),
    FetchUrlContent(FetchUrlContent),
    InstallSystem(InstallSystem),
    SmartUnpack(SmartUnpack),
    SearchCreatorStore(SearchCreatorStore),
    InsertAssets(InsertAssetsArgs),
    CreateScript(CreateScript),
    UpdateScript(UpdateScript),
    ReadScript(ReadScript),
}
#[tool_router]
impl RBXStudioServer {
    pub fn new(state: PackedState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Execute a Luau command or script snippet directly in Roblox Studio. Use this to modify the game state or query information not available via other tools."
    )]
    async fn run_command(
        &self,
        Parameters(args): Parameters<RunCommand>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::RunCommand(args))
            .await
    }

    #[tool(
        description = "Inserts a model from the Roblox marketplace into the workspace. Returns the inserted model name."
    )]
    async fn insert_model(
        &self,
        Parameters(args): Parameters<InsertModel>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::InsertModel(args))
            .await
    }

    #[tool(description = "Get all properties of a specific Roblox instance in Studio")]
    async fn get_instance_properties(
        &self,
        Parameters(args): Parameters<GetInstanceProperties>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetInstanceProperties(args))
            .await
    }

    #[tool(description = "Set a property on any Roblox instance")]
    async fn set_property(
        &self,
        Parameters(args): Parameters<SetProperty>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::SetProperty(args))
            .await
    }

    #[tool(description = "Get the Roblox instance hierarchy tree from Roblox Studio.")]
    async fn get_file_tree(
        &self,
        Parameters(args): Parameters<GetFileTree>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetFileTree(args))
            .await
    }

    #[tool(description = "Get available Roblox services and their children")]
    async fn get_services(
        &self,
        Parameters(args): Parameters<GetServices>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetServices(args))
            .await
    }

    #[tool(
        description = "Get complete game hierarchy. IMPORTANT: Use maxDepth parameter to explore deeper levels"
    )]
    async fn get_project_structure(
        &self,
        Parameters(args): Parameters<GetProjectStructure>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetProjectStructure(args))
            .await
    }

    #[tool(description = "Get child instances and their class types from a Roblox parent instance")]
    async fn get_instance_children(
        &self,
        Parameters(args): Parameters<GetInstanceChildren>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetInstanceChildren(args))
            .await
    }

    #[tool(description = "Search for Roblox instances by name, class type, or script content")]
    async fn search_files(
        &self,
        Parameters(args): Parameters<SearchFiles>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::SearchFiles(args))
            .await
    }

    #[tool(description = "Find instances by name, class, or properties")]
    async fn search_objects(
        &self,
        Parameters(args): Parameters<SearchObjects>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::SearchObjects(args))
            .await
    }

    #[tool(description = "Find objects with specific property values")]
    async fn search_by_property(
        &self,
        Parameters(args): Parameters<SearchByProperty>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::SearchByProperty(args))
            .await
    }

    #[tool(description = "Create a new Instance")]
    async fn create_object(
        &self,
        Parameters(args): Parameters<CreateObject>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::CreateObject(args))
            .await
    }

    #[tool(description = "Delete an Instance")]
    async fn delete_object(
        &self,
        Parameters(args): Parameters<DeleteObject>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::DeleteObject(args))
            .await
    }

    #[tool(description = "Set property on multiple instances")]
    async fn mass_set_property(
        &self,
        Parameters(args): Parameters<MassSetProperty>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::MassSetProperty(args))
            .await
    }

    #[tool(description = "Get property from multiple instances")]
    async fn mass_get_property(
        &self,
        Parameters(args): Parameters<MassGetProperty>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::MassGetProperty(args))
            .await
    }

    #[tool(description = "Set property using a mathematical formula")]
    async fn set_calculated_property(
        &self,
        Parameters(args): Parameters<SetCalculatedProperty>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::SetCalculatedProperty(args))
            .await
    }

    #[tool(description = "Set property relative to its current value")]
    async fn set_relative_property(
        &self,
        Parameters(args): Parameters<SetRelativeProperty>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::SetRelativeProperty(args))
            .await
    }

    #[tool(description = "Get information about the current place")]
    async fn get_place_info(
        &self,
        Parameters(args): Parameters<GetPlaceInfo>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetPlaceInfo(args))
            .await
    }

    #[tool(description = "Get API information for a specific class")]
    async fn get_class_info(
        &self,
        Parameters(args): Parameters<GetClassInfo>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetClassInfo(args))
            .await
    }

    #[tool(description = "Search the official Roblox Marketplace/Catalog")]
    async fn search_marketplace(
        &self,
        Parameters(args): Parameters<SearchMarketplace>,
    ) -> Result<CallToolResult, ErrorData> {
        // Rust-side implementation
        let query = args.query;
        let limit = args.limit.unwrap_or(10);
        // Category 11 = Models
        let category = match args.asset_type.as_deref() {
            Some("Audio") => 9, // Example category IDs needed, using Models (11) as default for now or finding documented ones
            _ => 11,            // Models
        };

        let client = reqwest::Client::new();
        // Using catalog API. Note: Many useful endpoints require auth.
        // Trying generic search items details
        let url = format!(
            "https://catalog.roblox.com/v1/search/items/details?Keyword={}&Category={}&Limit={}",
            query, category, limit
        );

        // This likely needs a robust implementation or proxy for stability, but we try direct first.
        let res = client.get(&url).send().await;

        match res {
            Ok(response) => {
                let text = response.text().await.unwrap_or_default();
                // Return raw JSON for now, or parse it?
                // Raw JSON is better for the LLM to inspect.
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Request failed: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Download a Roblox Asset (.rbxm) to the computer")]
    async fn download_asset(
        &self,
        Parameters(args): Parameters<DownloadAsset>,
    ) -> Result<CallToolResult, ErrorData> {
        let asset_id = args.asset_id;
        let file_name = args.file_name.unwrap_or_else(|| asset_id.to_string());
        let path = format!("assets/{}.rbxm", file_name);

        // Ensure assets dir exists
        let _ = tokio::fs::create_dir_all("assets").await;

        let client = reqwest::Client::new();
        let url = format!("https://assetdelivery.roblox.com/v1/asset?id={}", asset_id);

        let res = client.get(&url).send().await;

        match res {
            Ok(response) => {
                if response.status().is_success() {
                    let bytes = response
                        .bytes()
                        .await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    tokio::fs::write(&path, bytes)
                        .await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "Saved asset {} to {}",
                        asset_id, path
                    ))]))
                } else {
                    Ok(CallToolResult::error(vec![Content::text(format!(
                        "Failed to download: Status {}",
                        response.status()
                    ))]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Request failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Search the web using Tavily AI. Use 'basic' depth for quick results or 'advanced' for more comprehensive searches."
    )]
    async fn search_web_scripts(
        &self,
        Parameters(args): Parameters<SearchWebScripts>,
    ) -> Result<CallToolResult, ErrorData> {
        let api_key = std::env::var("TAVILY_API_KEY")
            .unwrap_or_else(|_| "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".to_string());

        // If still using placeholder/hardcoded, warn or proceed.
        // We kept the user's provided key as default for convenience,
        // but now they can override it in JSON.

        let client = reqwest::Client::new();

        // Use Tavily Search API with configurable depth
        // Research mode is disabled as it requires additional undocumented parameters
        let search_url = "https://api.tavily.com/search";
        let body = serde_json::json!({
            "query": args.query,
            "search_depth": args.depth.unwrap_or("basic".to_string()),
            "include_answer": true,
            "include_raw_content": true,
            "max_results": 5
        });

        let res = client
            .post(search_url)
            .bearer_auth(&api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                ErrorData::internal_error(format!("Search request failed: {}", e), None)
            })?;

        let text = res
            .text()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Extracts clean content from specific URLs using Tavily")]
    async fn tavily_extract(
        &self,
        Parameters(args): Parameters<TavilyExtract>,
    ) -> Result<CallToolResult, ErrorData> {
        let api_key = std::env::var("TAVILY_API_KEY")
            .unwrap_or_else(|_| "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".to_string());

        let client = reqwest::Client::new();

        let url = "https://api.tavily.com/extract";
        let body = serde_json::json!({
            "urls": args.urls,
        });

        let res = client
            .post(url)
            .bearer_auth(&api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                ErrorData::internal_error(format!("Extract request failed: {}", e), None)
            })?;

        let text = res
            .text()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Fetch raw text content from a URL (e.g. raw GitHub file)")]
    async fn fetch_url_content(
        &self,
        Parameters(args): Parameters<FetchUrlContent>,
    ) -> Result<CallToolResult, ErrorData> {
        let res = reqwest::get(&args.url)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let text = res
            .text()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Automated System Installer: Searches Marketplace for best match and installs it."
    )]
    async fn install_system(
        &self,
        Parameters(args): Parameters<InstallSystem>,
    ) -> Result<CallToolResult, ErrorData> {
        // 1. Search Marketplace
        let query = args.system_name.clone();
        let category = 11; // Models
        let url = format!(
            "https://catalog.roblox.com/v1/search/items/details?Keyword={}&Category={}&Limit=1",
            query, category
        );
        let client = reqwest::Client::new();
        let search_res = client
            .get(&url)
            .send()
            .await
            .map_err(|e| ErrorData::internal_error(format!("Search failed: {}", e), None))?;

        let search_json: serde_json::Value = search_res.json().await.map_err(|e| {
            ErrorData::internal_error(format!("Failed to parse search JSON: {}", e), None)
        })?;

        // Extract first asset ID
        let data =
            search_json
                .get("data")
                .and_then(|v| v.as_array())
                .ok_or(ErrorData::internal_error(
                    "Invalid search response format",
                    None,
                ))?;

        if let Some(first_item) = data.first() {
            let asset_id = first_item
                .get("id")
                .and_then(|v| v.as_u64())
                .ok_or(ErrorData::internal_error("Item has no ID", None))?;
            let name = first_item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");

            // 2. Download Asset
            let file_name = format!("{}_{}", name.replace(" ", "_"), asset_id);
            let path = format!("assets/{}.rbxm", file_name);
            let _ = tokio::fs::create_dir_all("assets").await;

            let download_url = format!("https://assetdelivery.roblox.com/v1/asset?id={}", asset_id);
            let download_res =
                client.get(&download_url).send().await.map_err(|e| {
                    ErrorData::internal_error(format!("Download failed: {}", e), None)
                })?;

            if !download_res.status().is_success() {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to download asset {}: Status {}",
                    asset_id,
                    download_res.status()
                ))]));
            }

            let bytes = download_res
                .bytes()
                .await
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            tokio::fs::write(&path, bytes)
                .await
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

            // 3. Insert into Studio
            // Convert struct to ToolArguments like generic_tool_run does, but we are inside the server.
            // We need to construct the ToolArguments message manually and send it.
            let insert_args = InsertModel {
                query: None,
                asset_id: Some(asset_id),
            };

            let tool_val = ToolArgumentValues::InsertModel(insert_args);
            let insert_result_raw = self.generic_tool_run(tool_val).await?;
            // insert_result is CallToolResult. We need the text content.
            // generic_tool_run returns CallToolResult.
            // The content[0].text is the model name.
            let model_name = insert_result_raw
                .content
                .first()
                .and_then(|c| match &c.raw {
                    rmcp::model::RawContent::Text(t) => Some(t.text.clone()),
                    _ => None,
                })
                .ok_or(ErrorData::internal_error(
                    "Failed to get model name from insert result",
                    None,
                ))?;

            // 4. Smart Unpack
            let unpack_args = SmartUnpack {
                target_name: model_name.clone(),
            };
            let unpack_val = ToolArgumentValues::SmartUnpack(unpack_args);
            let unpack_result = self.generic_tool_run(unpack_val).await?;

            // Return combined result
            Ok(CallToolResult::success(vec![Content::text(format!(
                 "Successfully installed System '{}' (Asset ID: {}).\nFile saved to: {}\nStudio Response: {}\nUnpack Response: {:?}", 
                 name, asset_id, path, model_name, unpack_result
             ))]))
        } else {
            Ok(CallToolResult::error(vec![Content::text(format!(
                "No results found for system: {}",
                query
            ))]))
        }
    }

    #[allow(dead_code)]
    async fn smart_unpack(
        &self,
        Parameters(args): Parameters<SmartUnpack>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::SmartUnpack(args))
            .await
    }

    #[tool(
        description = "Create a new Script, LocalScript, or ModuleScript with optional source code"
    )]
    async fn create_script(
        &self,
        Parameters(args): Parameters<CreateScript>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::CreateScript(args))
            .await
    }

    #[tool(description = "Update the source code of an existing script")]
    async fn update_script(
        &self,
        Parameters(args): Parameters<UpdateScript>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::UpdateScript(args))
            .await
    }

    #[tool(description = "Read the source code of a script")]
    async fn read_script(
        &self,
        Parameters(args): Parameters<ReadScript>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::ReadScript(args))
            .await
    }

    #[tool(
        description = "Search Roblox Creator Store/Toolbox for assets (models, scripts, audio, etc.) and optionally download them"
    )]
    async fn search_creator_store(
        &self,
        Parameters(args): Parameters<SearchCreatorStore>,
    ) -> Result<CallToolResult, ErrorData> {
        // Log the parameters for debugging
        let download_enabled = args.download.unwrap_or(false);
        tracing::info!(
            "search_creator_store called with query='{}', asset_type={:?}, limit={:?}, download={}",
            args.query,
            args.asset_type,
            args.limit,
            download_enabled
        );

        // Map asset type to searchCategoryType
        let asset_type = args.asset_type.unwrap_or("Model".to_string());
        let limit = args.limit.unwrap_or(10).min(100);

        // Build the search URL
        let url = format!(
            "https://apis.roblox.com/toolbox-service/v2/assets:search?searchCategoryType={}&query={}&maxPageSize={}",
            asset_type,
            urlencoding::encode(&args.query),
            limit
        );

        // Retry logic with exponential backoff
        let max_retries = 3;
        let mut search_json: Option<serde_json::Value> = None;
        let mut last_error = String::new();

        for attempt in 0..max_retries {
            if attempt > 0 {
                let backoff_ms = 1000 * (2_u64.pow(attempt as u32));
                tracing::warn!("Retry attempt {} after {}ms delay", attempt + 1, backoff_ms);
                tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
            }

            // Make the request with timeout
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| {
                    ErrorData::internal_error(format!("Failed to build client: {}", e), None)
                })?;

            let res = client.get(&url).send().await;

            match res {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        match response.json().await {
                            Ok(json) => {
                                search_json = Some(json);
                                break;
                            }
                            Err(e) => {
                                last_error = format!("Failed to parse response: {}", e);
                                tracing::error!(
                                    "Parse error on attempt {}: {}",
                                    attempt + 1,
                                    last_error
                                );
                                continue;
                            }
                        }
                    } else if status == reqwest::StatusCode::GATEWAY_TIMEOUT
                        || status == reqwest::StatusCode::REQUEST_TIMEOUT
                    {
                        last_error = format!("Request timeout ({})", status);
                        tracing::warn!("Timeout on attempt {}: {}", attempt + 1, last_error);
                        continue;
                    } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        last_error = "Rate limit exceeded".to_string();
                        tracing::warn!("Rate limited on attempt {}", attempt + 1);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        continue;
                    } else {
                        let error_text = response.text().await.unwrap_or_default();
                        last_error =
                            format!("Search failed with status {}: {}", status, error_text);
                        tracing::error!("Error on attempt {}: {}", attempt + 1, last_error);
                        if status.is_client_error()
                            && status != reqwest::StatusCode::TOO_MANY_REQUESTS
                        {
                            break;
                        }
                        continue;
                    }
                }
                Err(e) => {
                    last_error = format!("Search request failed: {}", e);
                    tracing::error!("Network error on attempt {}: {}", attempt + 1, last_error);
                    continue;
                }
            }
        }

        let search_json = match search_json {
            Some(json) => json,
            None => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed after {} attempts. Last error: {}\n\n💡 Tip: The Roblox Creator Store API may be experiencing issues or rate limiting. Try again in a few moments.",
                    max_retries, last_error
                ))]));
            }
        };

        // Extract results
        let assets = search_json
            .get("creatorStoreAssets")
            .and_then(|v| v.as_array())
            .ok_or(ErrorData::internal_error("Invalid response format", None))?;

        if assets.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No {} assets found for query: '{}'",
                asset_type, args.query
            ))]));
        }

        let mut result_text = format!("Found {} {} assets:\n\n", assets.len(), asset_type);
        let mut asset_ids = Vec::new();

        for (idx, asset) in assets.iter().enumerate() {
            let asset_obj = asset.get("asset");
            let id = asset_obj
                .and_then(|a| a.get("id"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let name = asset_obj
                .and_then(|a| a.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let description = asset_obj
                .and_then(|a| a.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let creator = asset
                .get("creator")
                .and_then(|c| c.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");

            result_text.push_str(&format!(
                "{}. {} (ID: {})\n   Creator: {}\n   Description: {}\n\n",
                idx + 1,
                name,
                id,
                creator,
                if description.len() > 100 {
                    &description[..100]
                } else {
                    description
                }
            ));

            // Collect asset IDs for insertion
            if download_enabled && id > 0 {
                asset_ids.push(id);
            }
        }

        // If download is enabled, insert assets into Roblox Studio via plugin
        if download_enabled && !asset_ids.is_empty() {
            result_text.push_str(&format!(
                "\n🎮 Inserting {} assets into Roblox Studio...\n",
                asset_ids.len()
            ));

            // Create InsertAssets command for the plugin
            let _insert_args = serde_json::json!({
                "InsertAssets": {
                    "asset_ids": asset_ids
                }
            });

            // Send to plugin via generic_tool_run
            let insert_result = self
                .generic_tool_run(ToolArgumentValues::InsertAssets(InsertAssetsArgs {
                    asset_ids: asset_ids.clone(),
                }))
                .await;

            match insert_result {
                Ok(tool_result) => {
                    // Extract text from the result
                    for content in &tool_result.content {
                        if let Some(text_content) = content.as_text() {
                            result_text.push_str(&format!("\n{}\n", text_content.text));
                        }
                    }
                }
                Err(e) => {
                    result_text.push_str(&format!("\n⚠ Failed to insert assets: {}\n", e));
                }
            }
        } else if !download_enabled {
            result_text.push_str("\n💡 Tip: To insert these assets into Roblox Studio, add 'download: true' to the parameters.\n");
            result_text
                .push_str("   Example: search_creator_store(query=\"sword\", download=true)\n");
        }

        Ok(CallToolResult::success(vec![Content::text(result_text)]))
    }

    async fn generic_tool_run(
        &self,
        args: ToolArgumentValues,
    ) -> Result<CallToolResult, ErrorData> {
        let (command, id) = ToolArguments::new(args);
        tracing::debug!("Running command: {:?}", command);
        let (tx, mut rx) = mpsc::unbounded_channel::<Result<String>>();
        let trigger = {
            let mut state = self.state.lock().await;
            state.process_queue.push_back(command);
            state.output_map.insert(id, tx);
            state.trigger.clone()
        };
        trigger
            .send(())
            .map_err(|e| ErrorData::internal_error(format!("Unable to trigger send {e}"), None))?;
        let result = rx
            .recv()
            .await
            .ok_or(ErrorData::internal_error("Couldn't receive response", None))?;
        {
            let mut state = self.state.lock().await;
            state.output_map.remove_entry(&id);
        }
        tracing::debug!("Sending to MCP: {result:?}");
        match result {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }
}

pub async fn request_handler(State(state): State<PackedState>) -> Result<impl IntoResponse> {
    let timeout = tokio::time::timeout(LONG_POLL_DURATION, async {
        loop {
            let mut waiter = {
                let mut state = state.lock().await;
                if let Some(task) = state.process_queue.pop_front() {
                    return Ok::<ToolArguments, Error>(task);
                }
                state.waiter.clone()
            };
            waiter.changed().await?
        }
    })
    .await;
    match timeout {
        Ok(result) => Ok(Json(result?).into_response()),
        _ => Ok((StatusCode::LOCKED, String::new()).into_response()),
    }
}

pub async fn response_handler(
    State(state): State<PackedState>,
    Json(payload): Json<RunCommandResponse>,
) -> Result<impl IntoResponse> {
    tracing::debug!("Received reply from studio {payload:?}");
    let mut state = state.lock().await;
    let tx = state
        .output_map
        .remove(&payload.id)
        .ok_or_eyre("Unknown ID")?;
    Ok(tx.send(Ok(payload.response))?)
}

pub async fn proxy_handler(
    State(state): State<PackedState>,
    Json(command): Json<ToolArguments>,
) -> Result<impl IntoResponse> {
    let id = command.id.ok_or_eyre("Got proxy command with no id")?;
    tracing::debug!("Received request to proxy {command:?}");
    let (tx, mut rx) = mpsc::unbounded_channel();
    {
        let mut state = state.lock().await;
        state.process_queue.push_back(command);
        state.output_map.insert(id, tx);
    }
    let response = rx.recv().await.ok_or_eyre("Couldn't receive response")??;
    {
        let mut state = state.lock().await;
        state.output_map.remove_entry(&id);
    }
    tracing::debug!("Sending back to dud: {response:?}");
    Ok(Json(RunCommandResponse { response, id }))
}

pub async fn dud_proxy_loop(state: PackedState, exit: Receiver<()>) {
    let client = reqwest::Client::new();

    let mut waiter = { state.lock().await.waiter.clone() };
    while exit.is_empty() {
        let entry = { state.lock().await.process_queue.pop_front() };
        if let Some(entry) = entry {
            let res = client
                .post(format!("http://127.0.0.1:{STUDIO_PLUGIN_PORT}/proxy"))
                .json(&entry)
                .send()
                .await;
            if let Ok(res) = res {
                let tx = {
                    state
                        .lock()
                        .await
                        .output_map
                        .remove(&entry.id.unwrap())
                        .unwrap()
                };
                let res = res
                    .json::<RunCommandResponse>()
                    .await
                    .map(|r| r.response)
                    .map_err(Into::into);
                tx.send(res).unwrap();
            } else {
                tracing::error!("Failed to proxy: {res:?}");
            };
        } else {
            waiter.changed().await.unwrap();
        }
    }
}
