use super::{Database, db_error};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub track_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PlaylistAction {
    List,
    Create {
        name: String,
    },
    Rename {
        playlist_id: String,
        name: String,
    },
    Delete {
        playlist_id: String,
    },
    Add {
        playlist_id: String,
        track_id: String,
    },
    Remove {
        playlist_id: String,
        track_id: String,
    },
}
fn valid_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a playlist name.".into());
    }
    Ok(name)
}
impl Database {
    pub fn playlists(&self) -> Result<Vec<Playlist>, String> {
        let mut statement = self
            .connection
            .prepare("SELECT id,name,created_at,updated_at FROM playlists ORDER BY created_at,id")
            .map_err(db_error)?;
        let mut playlists = statement
            .query_map([], |r| {
                Ok(Playlist {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    created_at: r.get(2)?,
                    updated_at: r.get(3)?,
                    track_ids: Vec::new(),
                })
            })
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        for playlist in &mut playlists {
            let mut statement = self.connection.prepare("SELECT pt.track_key FROM playlist_tracks pt JOIN tracks t ON t.path_key=pt.track_key WHERE pt.playlist_id=?1 AND pt.removed=0 ORDER BY pt.position").map_err(db_error)?;
            playlist.track_ids = statement
                .query_map([&playlist.id], |r| r.get(0))
                .map_err(db_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db_error)?;
        }
        Ok(playlists)
    }
    pub fn playlist_control(&mut self, action: PlaylistAction) -> Result<Vec<Playlist>, String> {
        if matches!(action, PlaylistAction::List) {
            return self.playlists();
        }
        let transaction = self.connection.transaction().map_err(db_error)?;
        match action {
            PlaylistAction::Create { name } => {
                transaction
                    .execute(
                        "INSERT INTO playlists(name) VALUES(?1)",
                        [valid_name(&name)?],
                    )
                    .map_err(db_error)?;
            }
            PlaylistAction::Rename { playlist_id, name } => {
                if transaction
                    .execute(
                        "UPDATE playlists SET name=?2,updated_at=unixepoch() WHERE id=?1",
                        params![playlist_id, valid_name(&name)?],
                    )
                    .map_err(db_error)?
                    == 0
                {
                    return Err(
                        "This playlist no longer exists. Reload playlists and try again.".into(),
                    );
                }
            }
            PlaylistAction::Delete { playlist_id } => {
                transaction
                    .execute("DELETE FROM playlists WHERE id=?1", [playlist_id])
                    .map_err(db_error)?;
            }
            PlaylistAction::Add {
                playlist_id,
                track_id,
            } => {
                transaction.execute("INSERT INTO playlist_tracks(playlist_id,track_key,position) VALUES(?1,?2,(SELECT coalesce(max(position)+1,0) FROM playlist_tracks WHERE playlist_id=?1)) ON CONFLICT(playlist_id,track_key) DO UPDATE SET position=excluded.position,removed=0 WHERE playlist_tracks.removed=1", params![playlist_id,track_id]).map_err(db_error)?;
                transaction
                    .execute(
                        "UPDATE playlists SET updated_at=unixepoch() WHERE id=?1",
                        [playlist_id],
                    )
                    .map_err(db_error)?;
            }
            PlaylistAction::Remove {
                playlist_id,
                track_id,
            } => {
                transaction.execute("UPDATE playlist_tracks SET removed=1 WHERE playlist_id=?1 AND track_key=?2", params![playlist_id,track_id]).map_err(db_error)?;
                transaction
                    .execute(
                        "UPDATE playlists SET updated_at=unixepoch() WHERE id=?1",
                        [playlist_id],
                    )
                    .map_err(db_error)?;
            }
            PlaylistAction::List => unreachable!(),
        }
        transaction.commit().map_err(db_error)?;
        self.playlists()
    }
    pub fn playlist_contains(&self, playlist_id: &str, track_id: &str) -> Result<bool, String> {
        self.connection.query_row("SELECT EXISTS(SELECT 1 FROM playlist_tracks WHERE playlist_id=?1 AND track_key=?2 AND removed=0)", params![playlist_id,track_id], |row| row.get(0)).map_err(db_error)
    }

    pub fn playlist_playback_neighbor(
        &self,
        playlist_id: &str,
        id: &str,
        previous: bool,
    ) -> Result<Option<String>, String> {
        // Removed memberships retain their position as a playback anchor. They are
        // excluded from lists/navigation and deleted with their playlist.
        let sql = if previous {
            "SELECT track_key FROM playlist_tracks WHERE playlist_id=?1 AND removed=0 AND position<(SELECT position FROM playlist_tracks WHERE playlist_id=?1 AND track_key=?2) ORDER BY position DESC LIMIT 1"
        } else {
            "SELECT track_key FROM playlist_tracks WHERE playlist_id=?1 AND removed=0 AND position>(SELECT position FROM playlist_tracks WHERE playlist_id=?1 AND track_key=?2) ORDER BY position LIMIT 1"
        };
        self.connection
            .query_row(sql, params![playlist_id, id], |r| r.get(0))
            .optional()
            .map_err(db_error)
    }
}
