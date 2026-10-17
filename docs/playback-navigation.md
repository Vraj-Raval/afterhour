# Playback navigation

## Up Next (Task 13)

The native audio worker owns the session-only explicit queue. Repeated songs are allowed; each queued occurrence has its own ID. Queue edits use a queue revision, independently of progress polling, to reject stale actions. Only successfully started entries are consumed.

- **Play next** inserts first; **+ Queue** appends. Neither starts or interrupts audio. With no current track, use **Start queue**.
- Next and natural completion consume queued entries before the underlying collection. After overrides, navigation resumes after the collection track that was playing when the override began, using live library/favorite/playlist membership.
- Previous after three seconds restarts the current song. Earlier Previous retraces queued playback and puts the song being left back at the front. Once back at the original collection song, normal collection navigation applies.
- Clearing or editing upcoming entries does not stop the current song. Clearing during an override still resumes the collection after the current song. A queue started without an underlying track ends when exhausted.
- Explicitly selecting a track preserves upcoming entries and establishes its collection as the new continuation. Viewing another screen does not change playback context.
- Missing queued files report an error and remain removable/retryable; automatic playback stops once instead of retrying forever or silently jumping elsewhere.
- Existing collection boundaries, Previous restart behavior, playback source-ID guards, and volume controls remain in force.

## Search and browsing (Task 14)

The top search field always searches the complete saved local library, by title, track artist, album, and album artist. Whitespace and capitalization are normalized. Terms can match across fields. Typing filters in-memory metadata; it never starts a filesystem scan. Clear or Escape restores the underlying view. Ctrl+K focuses search when no text field or modal is active. Choosing a navigation entry clears search.

Home shows actual collection counts, a small album shelf, and up to twelve available songs. Library shows every saved song, including unavailable files. Album identity uses normalized album title plus album artist (falling back to track artist); artist identity uses normalized track artist. Grouping handles missing metadata and normalizes Unicode, repeated whitespace, and capitalization. Album track order uses disc number, track number, title, and stable track ID. Artist playback sorts by album then the same track ordering.

Starting a search result, album, or artist creates a session playback selection of its complete ordered IDs, before pagination. Search uses the result order visible when playback starts. Typing a new query, switching screens, rescanning, or arriving on another page does not silently rewrite that selection. Select a song again to adopt refreshed results. Existing Library and Liked Songs contexts continue to use live SQLite membership, as do playlists.

Upcoming queue entries are preserved when starting any search or group result. They play before the saved selection resumes after its original anchor. No queued item changes the selection or leaks navigation into the full library. At a selection boundary playback stops using the established transport behavior.
