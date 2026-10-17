use super::{Backend, MusicTrack, Transport};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct QueueEntry {
    pub id: u64,
    pub track: MusicTrack,
}
#[derive(Default)]
pub(super) struct QueuePlayback {
    pub active: bool,
    pub anchor: Option<String>,
    pub history: Vec<String>,
}
impl<B: Backend> Transport<B> {
    pub(super) fn push_queue(&mut self, track: MusicTrack, first: bool) {
        self.next_queue_id += 1;
        let entry = QueueEntry {
            id: self.next_queue_id,
            track,
        };
        if first {
            self.state.queue.insert(0, entry);
        } else {
            self.state.queue.push(entry);
        }
        self.state.queue_revision += 1;
    }
    pub(super) fn edit_queue(
        &mut self,
        revision: u64,
        edit: impl FnOnce(&mut Vec<QueueEntry>) -> Result<(), String>,
    ) -> Result<(), String> {
        if revision != self.state.queue_revision {
            return Err("Up Next changed. Please try that queue action again.".into());
        }
        edit(&mut self.state.queue)?;
        self.state.queue_revision += 1;
        Ok(())
    }
}
