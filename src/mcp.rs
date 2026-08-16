use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};

use crate::notes::{
    add_tag_to_note, create_folder, create_note, create_tag, delete_note, empty_trash, get_folders,
    get_notes, get_tags, permanently_delete_note, remove_tag_from_note, restore_note,
    set_completed, set_pinned, set_reminder, update_note,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetNotesParams {
    #[serde(default = "default_limit")]
    limit: i32,
    search_query: Option<String>,
}

fn default_limit() -> i32 {
    10
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct CreateNoteParams {
    title: String,
    content: String,
    #[serde(default)]
    completed: bool,
    folder_id: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SetCompletedParams {
    id: i64,
    completed: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SetPinnedParams {
    id: i64,
    pinned: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct UpdateNoteParams {
    id: i64,
    title: Option<String>,
    content: Option<String>,
    folder_id: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct NoteIdParams {
    id: i64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct CreateFolderParams {
    name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct CreateTagParams {
    name: String,
    color_hex: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct NoteTagParams {
    note_id: i64,
    tag_id: i64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SetReminderParams {
    id: i64,
    /// Unix timestamp (seconds). Omit to clear the reminder.
    reminder_at: Option<i64>,
}

#[derive(Clone, Default)]
pub struct JotServer {
    // Read by the #[tool_handler]-generated dispatch, which dead_code analysis doesn't see through.
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl JotServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Reads notes and tasks from Jot, including folder and pinned state.")]
    fn get_notes(
        &self,
        Parameters(GetNotesParams {
            limit,
            search_query,
        }): Parameters<GetNotesParams>,
    ) -> String {
        match get_notes(limit, search_query.as_deref()) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(description = "Lists Jot's user-created folders (id and name).")]
    fn get_folders(&self) -> String {
        match get_folders() {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(description = "Creates a new folder in Jot.")]
    fn create_folder(
        &self,
        Parameters(CreateFolderParams { name }): Parameters<CreateFolderParams>,
    ) -> String {
        match create_folder(&name) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(description = "Lists Jot's tags (id, name, color).")]
    fn get_tags(&self) -> String {
        match get_tags() {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(description = "Creates a new tag in Jot, optionally with a hex color.")]
    fn create_tag(
        &self,
        Parameters(CreateTagParams { name, color_hex }): Parameters<CreateTagParams>,
    ) -> String {
        match create_tag(&name, color_hex.as_deref()) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(description = "Adds an existing tag to a note (see get_tags and get_notes for IDs).")]
    fn add_tag_to_note(
        &self,
        Parameters(NoteTagParams { note_id, tag_id }): Parameters<NoteTagParams>,
    ) -> String {
        match add_tag_to_note(note_id, tag_id) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(description = "Removes a tag from a note.")]
    fn remove_tag_from_note(
        &self,
        Parameters(NoteTagParams { note_id, tag_id }): Parameters<NoteTagParams>,
    ) -> String {
        match remove_tag_from_note(note_id, tag_id) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(
        description = "Creates a new note in Jot. Notes can optionally be created already checked off via `completed` and/or filed into a folder via `folder_id` (see get_folders)."
    )]
    fn create_note(
        &self,
        Parameters(CreateNoteParams {
            title,
            content,
            completed,
            folder_id,
        }): Parameters<CreateNoteParams>,
    ) -> String {
        match create_note(&title, &content, completed, folder_id) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(description = "Checks or unchecks an existing note in Jot by ID.")]
    fn set_completed(
        &self,
        Parameters(SetCompletedParams { id, completed }): Parameters<SetCompletedParams>,
    ) -> String {
        match set_completed(id, completed) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(description = "Pins or unpins an existing note in Jot by ID.")]
    fn set_pinned(
        &self,
        Parameters(SetPinnedParams { id, pinned }): Parameters<SetPinnedParams>,
    ) -> String {
        match set_pinned(id, pinned) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(
        description = "Sets or clears a reminder date/time on a note by ID. Pass `reminder_at` as a Unix timestamp, or omit it to clear the reminder."
    )]
    fn set_reminder(
        &self,
        Parameters(SetReminderParams { id, reminder_at }): Parameters<SetReminderParams>,
    ) -> String {
        match set_reminder(id, reminder_at) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(
        description = "Updates the title, content, and/or folder of an existing note in Jot by ID. Omitted fields are left unchanged."
    )]
    fn update_note(
        &self,
        Parameters(UpdateNoteParams {
            id,
            title,
            content,
            folder_id,
        }): Parameters<UpdateNoteParams>,
    ) -> String {
        match update_note(id, title.as_deref(), content.as_deref(), folder_id) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(
        description = "Moves a note to Trash in Jot by ID (soft delete, matching the app's own delete behavior)."
    )]
    fn delete_note(&self, Parameters(NoteIdParams { id }): Parameters<NoteIdParams>) -> String {
        match delete_note(id) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(description = "Restores a note from Trash back to unfiled, by ID.")]
    fn restore_note(&self, Parameters(NoteIdParams { id }): Parameters<NoteIdParams>) -> String {
        match restore_note(id) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(
        description = "Permanently deletes a note by ID. The note must already be in Trash (use delete_note first)."
    )]
    fn permanently_delete_note(
        &self,
        Parameters(NoteIdParams { id }): Parameters<NoteIdParams>,
    ) -> String {
        match permanently_delete_note(id) {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }

    #[tool(description = "Permanently deletes every note currently in Trash.")]
    fn empty_trash(&self) -> String {
        match empty_trash() {
            Ok(res) => res,
            Err(e) => format!("Database error: {}", e),
        }
    }
}

#[tool_handler]
impl ServerHandler for JotServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::new(ServerCapabilities::builder().enable_tools().build());
        info.server_info = rmcp::model::Implementation::new("jot-mcp", env!("CARGO_PKG_VERSION"));
        info
    }
}

pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let service = JotServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
