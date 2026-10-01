use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveNote {
    pub id: Option<i64>,
    pub expected_version: Option<i64>,
    pub title: String,
    pub body: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteNote {
    pub id: i64,
    pub expected_version: i64,
}
fn error(e: rusqlite::Error) -> String {
    e.to_string()
}
pub fn read(c: &Connection) -> Result<Vec<Note>, String> {
    let mut query=c.prepare("SELECT id,title,body,version,created_at,updated_at FROM notes ORDER BY updated_at DESC,id DESC").map_err(error)?;
    let rows = query
        .query_map([], |r| {
            Ok(Note {
                id: r.get(0)?,
                title: r.get(1)?,
                body: r.get(2)?,
                version: r.get(3)?,
                created_at: r.get(4)?,
                updated_at: r.get(5)?,
            })
        })
        .map_err(error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(error)
}
pub fn save(c: &Connection, input: SaveNote) -> Result<(), String> {
    let title = input.title.trim();
    if title.is_empty() || title.chars().count() > 120 {
        return Err("Note titles must have 1–120 characters.".into());
    }
    if input.body.chars().count() > 50000 {
        return Err("Notes must be at most 50,000 characters.".into());
    }
    match (input.id, input.expected_version) {
        (None, None) => {
            c.execute(
                "INSERT INTO notes(title,body,version) VALUES(?1,?2,1)",
                params![title, input.body],
            )
            .map_err(error)?;
        }
        (Some(id), Some(version)) => {
            let old = c
                .query_row(
                    "SELECT title,body,version FROM notes WHERE id=?1",
                    [id],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, i64>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(error)?
                .ok_or("Note not found. Reload the notepad.")?;
            if old.2 != version {
                return Err("This note changed in another window. Reload it before saving.".into());
            }
            if old.0 == title && old.1 == input.body {
                return Ok(());
            }
            c.execute("UPDATE notes SET title=?1,body=?2,version=version+1,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?3",params![title,input.body,id]).map_err(error)?;
        }
        _ => return Err("Invalid note version.".into()),
    }
    Ok(())
}
pub fn delete(c: &Connection, input: DeleteNote) -> Result<(), String> {
    if c.execute(
        "DELETE FROM notes WHERE id=?1 AND version=?2",
        params![input.id, input.expected_version],
    )
    .map_err(error)?
        != 1
    {
        return Err(
            "This note changed or was deleted in another window. Reload it before deleting.".into(),
        );
    }
    Ok(())
}
