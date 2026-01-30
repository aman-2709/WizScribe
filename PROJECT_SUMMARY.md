# WizScribe Project Summary

## Overview
WizScribe is a complete AI meeting notes application for Linux built with:
- **Frontend**: SvelteKit + TailwindCSS + TypeScript
- **Backend**: Rust + Tauri
- **Audio**: cpal for cross-platform audio capture
- **Transcription**: Whisper.cpp via whisper-rs
- **AI**: OpenAI GPT-4 / Anthropic Claude

## Project Structure

### Frontend (src/)
```
src/
├── app.css                    # Tailwind CSS configuration
├── app.html                   # HTML template
├── lib/
│   ├── api.ts                 # Tauri API wrapper functions
│   ├── components/
│   │   ├── AIChat.svelte      # AI chat interface
│   │   ├── AudioPlayer.svelte # Audio playback controls
│   │   ├── MeetingList.svelte # Meeting list sidebar
│   │   ├── NoteEditor.svelte  # Rich text note editor
│   │   ├── RecordingControls.svelte # Record/pause/stop buttons
│   │   └── TranscriptViewer.svelte  # Transcript display with AI summary
│   ├── stores/
│   │   └── index.ts           # Svelte stores for state management
│   └── types/
│       └── index.ts           # TypeScript type definitions
└── routes/
    ├── +layout.svelte         # Main app layout with sidebar
    ├── +page.svelte           # Home/dashboard page
    ├── meeting/[id]/
    │   └── +page.svelte       # Meeting detail page
    └── settings/
        └── +page.svelte       # App settings page
```

### Backend (src-tauri/src/)
```
src-tauri/src/
├── main.rs                    # Tauri command handlers & app setup
├── lib.rs                     # Library exports
├── audio.rs                   # Audio recording with cpal
├── db.rs                      # SQLite database with sqlx
├── whisper.rs                 # Whisper.cpp transcription
└── ai.rs                      # OpenAI/Anthropic API integration
```

## Key Features

### 1. Audio Recording
- Records from default microphone using cpal
- Saves audio as WAV files in app data directory
- Supports start/pause/stop functionality

### 2. Database (SQLite)
**Tables:**
- `meetings`: id, title, created_at, audio_path, duration, transcript, summary
- `notes`: id, meeting_id, content, timestamps_json, updated_at
- `templates`: id, name, structure_json, created_at

### 3. Transcription
- Uses Whisper.cpp via whisper-rs bindings
- Supports local transcription (no cloud required)
- Timestamps for each segment

### 4. AI Integration
- OpenAI GPT-4 support
- Anthropic Claude 3 support
- Meeting summaries
- Q&A on transcripts
- Action item extraction

### 5. Note Editor
- Rich text with markdown-like formatting
- Template support (Standard, Sprint Review, 1:1, Interview)
- Timestamp insertion
- Auto-save

### 6. Export
- Markdown export with meeting metadata
- Includes transcript, notes, and summary

## Installation

```bash
# Install dependencies
npm install

# Run development server
npm run tauri dev

# Build for production
npm run tauri build
```

## Setup Whisper Model

```bash
# Run the setup script
./scripts/setup-whisper.sh

# Or manually download:
mkdir -p ~/.local/share/wizscribe/models
curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin \
  -o ~/.local/share/wizscribe/models/ggml-base.en.bin
```

## Configuration

1. Open Settings in the app
2. Choose AI provider (OpenAI or Anthropic)
3. Enter your API key
4. API keys are stored locally only

## Dependencies

### Rust Crates
- tauri, tauri-plugin-opener, tauri-plugin-dialog, tauri-plugin-fs
- tokio, chrono, uuid
- sqlx (SQLite)
- whisper-rs
- reqwest
- cpal (audio capture)
- hound (WAV file handling)
- rubato (audio resampling)

### NPM Packages
- @tauri-apps/api, @tauri-apps/cli
- svelte, @sveltejs/kit
- tailwindcss, @tailwindcss/typography
- lucide-svelte (icons)

## Development Notes

- The app uses SPA mode (ssr: false) for Tauri compatibility
- Audio recordings are stored in the app data directory
- Database is SQLite with migrations handled automatically
- All AI processing requires API key configuration

## License
MIT
