# Afterhours

**Your music. Your library. Your space.**

Afterhours is a local-first desktop music player built for people who want a dedicated space for their personal music collections. Browse your library, organize your favorite tracks, create playlists, manage your playback queue, and enjoy your music through a modern, Spotify-inspired desktop interface.

Built with Tauri, React, TypeScript, and Rust, Afterhours combines a web-based UI with native desktop capabilities.

## Features

### Music Library

- Select local music folders using the native folder picker.
- Scan folders recursively to discover music files.
- Extract track metadata and available album artwork.
- Browse your collection through Library, Albums, Artists, and Home views.
- Search tracks by title, artist, and album.

### Audio Playback

- Play local audio files through the native playback engine.
- Pause and resume playback.
- Seek to any position in a track.
- Navigate between tracks with Next and Previous.
- Automatically advance when a track finishes.
- Control playback volume and mute audio.
- Navigate within the active library collection, liked songs, or playlist.

### Favorites and Playlists

- Save and remove favorite tracks in Liked Songs.
- Create, rename, and delete custom playlists.
- Add tracks to playlists and remove them when needed.
- Preserve playlist track ordering.
- Keep favorites and playlists associated with your local music library.

### Playback Queue

- View upcoming tracks.
- Add tracks to the queue.
- Play a track next.
- Remove and reorder upcoming tracks.
- Manage the queue without replacing your entire music library.

### Desktop Experience

- Dark interface with Spotify-inspired styling and green accents.
- Persistent playback controls.
- Dedicated library and Now Playing panels.
- Artwork-focused music browsing.
- Local-first operation without requiring a streaming account.

### Local Data Persistence

- Store library metadata and application data using SQLite.
- Restore saved library information when reopening the app.
- Persist favorites and playlists across restarts.
- Avoid duplicating tracks during library rescans.

> Afterhours is designed for local music playback. It is an independent project and is not affiliated with, endorsed by, or sponsored by Spotify.

## Tech Stack

| Technology   | Purpose                                          |
| ------------ | ------------------------------------------------ |
| Tauri 2      | Native desktop application framework             |
| React        | User interface                                   |
| TypeScript   | Type-safe frontend development                   |
| Vite         | Frontend development server and build tooling    |
| Rust         | Native application logic and desktop integration |
| SQLite       | Persistent local application data                |
| HTML and CSS | Interface structure and styling                  |

The exact audio, metadata, and database libraries are determined by the project's implementation.

## Requirements

Before running Afterhours locally, make sure you have:

- Node.js and npm.
- Rust and Cargo.
- The platform-specific prerequisites for Tauri 2.
- Git.

For Windows development, follow the official Tauri prerequisites guide:

https://v2.tauri.app/start/prerequisites/

## Getting Started

### 1. Clone the repository

```bash
git clone <YOUR_REPOSITORY_URL>
cd afterhours
```

Replace `<YOUR_REPOSITORY_URL>` with your Git repository URL.

### 2. Install dependencies

```bash
npm install
```

### 3. Start the development application

```bash
npm run tauri dev
```

This starts the frontend development server and launches the native Afterhours desktop application through Tauri.

To run only the frontend development server, use:

```bash
npm run dev
```

Note that frontend-only mode may not support functionality that requires native Tauri commands.

## Production Build

Build the desktop application using:

```bash
npm run tauri build
```

The resulting installer or executable artifacts depend on the platform and Tauri configuration. Check the build output and project configuration for the exact artifact locations.

## Using Afterhours

1. Launch Afterhours.
2. Select a folder containing your local music.
3. Let the application scan the folder and extract available metadata.
4. Browse your library, albums, and artists.
5. Play tracks and control playback from the persistent player.
6. Favorite tracks or create playlists to organize your collection.
7. Use search and the playback queue to find and manage what you want to hear.

Your results depend on the audio files and metadata available in your local collection.

## Project Structure

The exact structure may evolve as the project develops. The application generally separates the following responsibilities:

- **Frontend:** React components, navigation, music views, and player controls.
- **Native backend:** Rust logic and Tauri commands.
- **Local database:** SQLite-backed library and organization data.
- **Desktop integration:** Native folder selection, file access, and application packaging.

See the source tree for the current implementation.

## Development and Testing

Before submitting changes, run the checks available in the repository.

For example:

```bash
npm run build
```

Run Rust checks from the directory containing `Cargo.toml`:

```bash
cargo check
cargo test
```

The exact commands available may vary with the project configuration. Consult `package.json` and the Rust workspace configuration before running them.

## Roadmap

Potential future improvements include:

- Keyboard shortcuts and media-key support.
- Improved library organization and sorting.
- More playback preferences and customization.
- Additional desktop integration and usability improvements.
- Packaging and release distribution.

## Contributing

Afterhours is a personal software project under active development. Bug reports, ideas, and improvements are welcome.

When contributing, keep changes focused, preserve existing playback behavior, and test library persistence and navigation.

## License

No license has been specified yet. Unless a license is added to this repository, do not assume the code is available for unrestricted reuse or redistribution.
