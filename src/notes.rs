use rusqlite::{OptionalExtension, Result as SqliteResult, Row, params};
use serde::Serialize;
use uuid::Uuid;

use crate::db::{APPLE_EPOCH_OFFSET, current_apple_time, get_connection};

#[derive(Serialize)]
pub struct Note {
    id: i64,
    title: Option<String>,
    content: Option<String>,
    completed: bool,
    pinned: bool,
    folder: Option<String>,
    tags: Vec<String>,
    created_at: u64,
}

#[derive(Serialize)]
pub struct Folder {
    id: i64,
    name: Option<String>,
}

#[derive(Serialize)]
pub struct Tag {
    id: i64,
    name: Option<String>,
    color_hex: Option<String>,
}

fn row_to_note(row: &Row) -> SqliteResult<Note> {
    let apple_time: f64 = row.get(6).unwrap_or(0.0);
    let tags_str: Option<String> = row.get(7)?;
    let tags = tags_str
        .map(|s| s.split(',').map(|t| t.to_string()).collect())
        .unwrap_or_default();
    Ok(Note {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        completed: row.get::<_, i32>(3).unwrap_or(0) != 0,
        pinned: row.get::<_, i32>(4).unwrap_or(0) != 0,
        folder: row.get(5)?,
        tags,
        created_at: (apple_time as u64) + APPLE_EPOCH_OFFSET,
    })
}

pub fn get_notes(limit: i32, search_query: Option<&str>) -> SqliteResult<String> {
    let conn = get_connection()?;
    let base_query = "SELECT n.Z_PK, n.ZTITLE, n.ZTEXT, n.ZCOMPLETED, n.ZPINNED, f.ZNAME, n.ZCREATED_AT, \
                       (SELECT GROUP_CONCAT(t.ZNAME, ',') FROM Z_2TAGS jt JOIN ZCDTAG t ON jt.Z_3TAGS = t.Z_PK WHERE jt.Z_2NOTES = n.Z_PK) \
                       FROM ZCDNOTE n LEFT JOIN ZCDFOLDER f ON n.ZFOLDER = f.Z_PK";

    let notes: Vec<Note> = if let Some(query_str) = search_query {
        let sql = format!(
            "{} WHERE n.ZTITLE LIKE ?1 OR n.ZTEXT LIKE ?2 ORDER BY n.ZCREATED_AT DESC LIMIT ?3",
            base_query
        );
        let search_param = format!("%{}%", query_str);
        let mut stmt = conn.prepare(&sql)?;
        stmt.query_map(params![search_param, search_param, limit], row_to_note)?
            .collect::<SqliteResult<Vec<_>>>()?
    } else {
        let sql = format!("{} ORDER BY n.ZCREATED_AT DESC LIMIT ?1", base_query);
        let mut stmt = conn.prepare(&sql)?;
        stmt.query_map(params![limit], row_to_note)?
            .collect::<SqliteResult<Vec<_>>>()?
    };

    Ok(serde_json::to_string_pretty(&notes).unwrap())
}

pub fn get_folders() -> SqliteResult<String> {
    let conn = get_connection()?;
    let mut stmt =
        conn.prepare("SELECT Z_PK, ZNAME FROM ZCDFOLDER WHERE ZISSYSTEM = 0 ORDER BY ZNAME")?;
    let folders: Vec<Folder> = stmt
        .query_map([], |row| {
            Ok(Folder {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(serde_json::to_string_pretty(&folders).unwrap())
}

pub fn create_folder(name: &str) -> SqliteResult<String> {
    let conn = get_connection()?;
    let z_ent: i32 = conn.query_row(
        "SELECT Z_ENT FROM Z_PRIMARYKEY WHERE Z_NAME = 'CDFolder'",
        [],
        |row| row.get(0),
    )?;
    let current_max: i64 = conn.query_row(
        "SELECT Z_MAX FROM Z_PRIMARYKEY WHERE Z_NAME = 'CDFolder'",
        [],
        |row| row.get(0),
    )?;
    let z_pk = current_max + 1;

    conn.execute(
        "UPDATE Z_PRIMARYKEY SET Z_MAX = ?1 WHERE Z_NAME = 'CDFolder'",
        params![z_pk],
    )?;

    let apple_time = current_apple_time();
    let z_id = Uuid::new_v4();

    conn.execute(
        "INSERT INTO ZCDFOLDER (Z_PK, Z_ENT, Z_OPT, ZISDYNAMIC, ZISPARTICIPANT, ZISSHARED, ZISSYSTEM, ZORDER, ZSHAREDBYME, ZCREATED_AT, ZNAME, ZID) VALUES (?1, ?2, 1, 0, 0, 0, 0, 0, 0, ?3, ?4, ?5)",
        params![z_pk, z_ent, apple_time, name, z_id.as_bytes()],
    )?;
    Ok(format!(
        "Successfully created folder: '{}' (ID: {})",
        name, z_pk
    ))
}

pub fn create_note(
    title: &str,
    content: &str,
    completed: bool,
    folder_id: Option<i64>,
) -> SqliteResult<String> {
    let conn = get_connection()?;
    let z_ent: i32 = conn.query_row(
        "SELECT Z_ENT FROM Z_PRIMARYKEY WHERE Z_NAME = 'CDNote'",
        [],
        |row| row.get(0),
    )?;
    let current_max: i64 = conn.query_row(
        "SELECT Z_MAX FROM Z_PRIMARYKEY WHERE Z_NAME = 'CDNote'",
        [],
        |row| row.get(0),
    )?;
    let z_pk = current_max + 1;

    conn.execute(
        "UPDATE Z_PRIMARYKEY SET Z_MAX = ?1 WHERE Z_NAME = 'CDNote'",
        params![z_pk],
    )?;

    let apple_time = current_apple_time();
    let z_id = Uuid::new_v4();
    let completed_val = if completed { 1 } else { 0 };

    conn.execute(
        "INSERT INTO ZCDNOTE (Z_PK, Z_ENT, Z_OPT, ZCLOUDVERSION, ZTITLE, ZTEXT, ZCOMPLETED, ZFOLDER, ZCREATED_AT, ZEDITED_AT, ZID) VALUES (?1, ?2, 1, 1, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![z_pk, z_ent, title, content, completed_val, folder_id, apple_time, apple_time, z_id.as_bytes()],
    )?;
    Ok(format!("Successfully created: '{}' (ID: {})", title, z_pk))
}

pub fn set_completed(id: i64, completed: bool) -> SqliteResult<String> {
    let conn = get_connection()?;
    let apple_time = current_apple_time();
    let completed_val = if completed { 1 } else { 0 };

    let rows_changed = conn.execute(
        "UPDATE ZCDNOTE SET ZCOMPLETED = ?1, ZEDITED_AT = ?2, Z_OPT = COALESCE(Z_OPT, 0) + 1 WHERE Z_PK = ?3",
        params![completed_val, apple_time, id],
    )?;

    if rows_changed == 0 {
        return Ok(format!("No note found with ID {}", id));
    }
    let verb = if completed {
        "completed"
    } else {
        "uncompleted"
    };
    Ok(format!("Successfully marked note {} as {}", id, verb))
}

pub fn set_pinned(id: i64, pinned: bool) -> SqliteResult<String> {
    let conn = get_connection()?;
    let apple_time = current_apple_time();
    let pinned_val = if pinned { 1 } else { 0 };

    let rows_changed = conn.execute(
        "UPDATE ZCDNOTE SET ZPINNED = ?1, ZEDITED_AT = ?2, Z_OPT = COALESCE(Z_OPT, 0) + 1 WHERE Z_PK = ?3",
        params![pinned_val, apple_time, id],
    )?;

    if rows_changed == 0 {
        return Ok(format!("No note found with ID {}", id));
    }
    let verb = if pinned { "pinned" } else { "unpinned" };
    Ok(format!("Successfully {} note {}", verb, id))
}

pub fn update_note(
    id: i64,
    title: Option<&str>,
    content: Option<&str>,
    folder_id: Option<i64>,
) -> SqliteResult<String> {
    if title.is_none() && content.is_none() && folder_id.is_none() {
        return Ok("No fields provided to update".to_string());
    }
    let conn = get_connection()?;
    let apple_time = current_apple_time();

    let mut clauses: Vec<&str> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(t) = title {
        clauses.push("ZTITLE = ?");
        values.push(Box::new(t.to_string()));
    }
    if let Some(c) = content {
        clauses.push("ZTEXT = ?");
        values.push(Box::new(c.to_string()));
    }
    if let Some(f) = folder_id {
        clauses.push("ZFOLDER = ?");
        values.push(Box::new(f));
    }
    clauses.push("ZEDITED_AT = ?");
    values.push(Box::new(apple_time));
    clauses.push("Z_OPT = COALESCE(Z_OPT, 0) + 1");
    values.push(Box::new(id));

    let sql = format!("UPDATE ZCDNOTE SET {} WHERE Z_PK = ?", clauses.join(", "));
    let param_refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    let rows_changed = conn.execute(&sql, param_refs.as_slice())?;

    if rows_changed == 0 {
        return Ok(format!("No note found with ID {}", id));
    }
    Ok(format!("Successfully updated note {}", id))
}

fn trash_folder_id(conn: &rusqlite::Connection) -> SqliteResult<i64> {
    conn.query_row(
        "SELECT Z_PK FROM ZCDFOLDER WHERE ZISSYSTEM = 1 AND ZNAME = 'Trash' ORDER BY Z_PK ASC LIMIT 1",
        [],
        |row| row.get(0),
    )
}

/// Moves a note to Jot's system Trash folder rather than hard-deleting the
/// row, matching how the app itself deletes notes (soft-delete).
pub fn delete_note(id: i64) -> SqliteResult<String> {
    let conn = get_connection()?;
    let trash_folder = trash_folder_id(&conn)?;
    let apple_time = current_apple_time();

    let rows_changed = conn.execute(
        "UPDATE ZCDNOTE SET ZFOLDER = ?1, ZEDITED_AT = ?2, Z_OPT = COALESCE(Z_OPT, 0) + 1 WHERE Z_PK = ?3",
        params![trash_folder, apple_time, id],
    )?;

    if rows_changed == 0 {
        return Ok(format!("No note found with ID {}", id));
    }
    Ok(format!("Successfully moved note {} to Trash", id))
}

/// Moves a note out of Trash back to unfiled (no folder), the same place a
/// freshly created note starts out.
pub fn restore_note(id: i64) -> SqliteResult<String> {
    let conn = get_connection()?;
    let apple_time = current_apple_time();

    let rows_changed = conn.execute(
        "UPDATE ZCDNOTE SET ZFOLDER = NULL, ZEDITED_AT = ?1, Z_OPT = COALESCE(Z_OPT, 0) + 1 WHERE Z_PK = ?2",
        params![apple_time, id],
    )?;

    if rows_changed == 0 {
        return Ok(format!("No note found with ID {}", id));
    }
    Ok(format!("Successfully restored note {} from Trash", id))
}

/// Hard-deletes a note. Only allowed for notes already in Trash, mirroring
/// the app's own two-step delete (Trash, then Empty Trash).
pub fn permanently_delete_note(id: i64) -> SqliteResult<String> {
    let conn = get_connection()?;
    let trash_folder = trash_folder_id(&conn)?;

    let current_folder: Option<Option<i64>> = conn
        .query_row(
            "SELECT ZFOLDER FROM ZCDNOTE WHERE Z_PK = ?1",
            params![id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()?;

    match current_folder {
        None => Ok(format!("No note found with ID {}", id)),
        Some(Some(folder)) if folder == trash_folder => {
            conn.execute("DELETE FROM ZCDNOTE WHERE Z_PK = ?1", params![id])?;
            Ok(format!("Successfully permanently deleted note {}", id))
        }
        _ => Ok(format!(
            "Note {} is not in Trash — move it there first with delete_note before permanently deleting it",
            id
        )),
    }
}

/// Permanently deletes every note currently in Trash.
pub fn empty_trash() -> SqliteResult<String> {
    let conn = get_connection()?;
    let trash_folder = trash_folder_id(&conn)?;
    let rows_changed = conn.execute(
        "DELETE FROM ZCDNOTE WHERE ZFOLDER = ?1",
        params![trash_folder],
    )?;
    Ok(format!(
        "Successfully permanently deleted {} note(s) from Trash",
        rows_changed
    ))
}

pub fn get_tags() -> SqliteResult<String> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare("SELECT Z_PK, ZNAME, ZCOLORHEX FROM ZCDTAG ORDER BY ZNAME")?;
    let tags: Vec<Tag> = stmt
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color_hex: row.get(2)?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(serde_json::to_string_pretty(&tags).unwrap())
}

pub fn create_tag(name: &str, color_hex: Option<&str>) -> SqliteResult<String> {
    let conn = get_connection()?;
    let z_ent: i32 = conn.query_row(
        "SELECT Z_ENT FROM Z_PRIMARYKEY WHERE Z_NAME = 'CDTag'",
        [],
        |row| row.get(0),
    )?;
    let current_max: i64 = conn.query_row(
        "SELECT Z_MAX FROM Z_PRIMARYKEY WHERE Z_NAME = 'CDTag'",
        [],
        |row| row.get(0),
    )?;
    let z_pk = current_max + 1;

    conn.execute(
        "UPDATE Z_PRIMARYKEY SET Z_MAX = ?1 WHERE Z_NAME = 'CDTag'",
        params![z_pk],
    )?;

    let z_id = Uuid::new_v4();
    conn.execute(
        "INSERT INTO ZCDTAG (Z_PK, Z_ENT, Z_OPT, ZNAME, ZCOLORHEX, ZID) VALUES (?1, ?2, 1, ?3, ?4, ?5)",
        params![z_pk, z_ent, name, color_hex, z_id.as_bytes()],
    )?;
    Ok(format!(
        "Successfully created tag: '{}' (ID: {})",
        name, z_pk
    ))
}

pub fn add_tag_to_note(note_id: i64, tag_id: i64) -> SqliteResult<String> {
    let conn = get_connection()?;
    conn.execute(
        "INSERT OR IGNORE INTO Z_2TAGS (Z_2NOTES, Z_3TAGS) VALUES (?1, ?2)",
        params![note_id, tag_id],
    )?;
    Ok(format!(
        "Successfully tagged note {} with tag {}",
        note_id, tag_id
    ))
}

pub fn remove_tag_from_note(note_id: i64, tag_id: i64) -> SqliteResult<String> {
    let conn = get_connection()?;
    conn.execute(
        "DELETE FROM Z_2TAGS WHERE Z_2NOTES = ?1 AND Z_3TAGS = ?2",
        params![note_id, tag_id],
    )?;
    Ok(format!(
        "Successfully removed tag {} from note {}",
        tag_id, note_id
    ))
}

/// `reminder_at` is a Unix timestamp; `None` clears the reminder. The
/// identifier format (`note_reminder_<uppercase UUID>`) matches what Jot
/// itself writes when a reminder is set via the app.
pub fn set_reminder(id: i64, reminder_at: Option<i64>) -> SqliteResult<String> {
    let conn = get_connection()?;
    let apple_time = current_apple_time();

    let rows_changed = match reminder_at {
        Some(unix_time) => {
            let reminder_apple_time = (unix_time as f64) - (APPLE_EPOCH_OFFSET as f64);
            let identifier = format!(
                "note_reminder_{}",
                Uuid::new_v4().to_string().to_uppercase()
            );
            conn.execute(
                "UPDATE ZCDNOTE SET ZREMINDERDATE = ?1, ZREMINDERIDENTIFIER = ?2, ZEDITED_AT = ?3, Z_OPT = COALESCE(Z_OPT, 0) + 1 WHERE Z_PK = ?4",
                params![reminder_apple_time, identifier, apple_time, id],
            )?
        }
        None => conn.execute(
            "UPDATE ZCDNOTE SET ZREMINDERDATE = NULL, ZREMINDERIDENTIFIER = NULL, ZEDITED_AT = ?1, Z_OPT = COALESCE(Z_OPT, 0) + 1 WHERE Z_PK = ?2",
            params![apple_time, id],
        )?,
    };

    if rows_changed == 0 {
        return Ok(format!("No note found with ID {}", id));
    }
    let verb = if reminder_at.is_some() {
        "set"
    } else {
        "cleared"
    };
    Ok(format!("Successfully {} reminder for note {}", verb, id))
}
