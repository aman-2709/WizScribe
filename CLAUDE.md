# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WizScribe is an AI-powered meeting notes application for Linux built with Tauri (Rust backend) and SvelteKit (TypeScript frontend). It captures audio, transcribes using local Whisper.cpp, and provides AI summaries via OpenAI/Anthropic APIs.

## Development Commands

```bash
# Start development (frontend + Tauri backend with hot reload)
npm run tauri dev

# Build production app
npm run tauri build

# Frontend only (no Tauri)
npm run dev           # Dev server on port 1420
npm run build         # Production build
npm run preview       # Preview production build

# Type checking
npm run check         # One-time check
npm run check:watch   # Watch mode
```

## Architecture

### Frontend (`src/`)
- **SvelteKit SPA** with static adapter (required for Tauri)
- **State management**: Svelte writable stores in `src/lib/stores/index.ts`
- **Tauri IPC**: All backend calls go through `src/lib/api.ts` which wraps `@tauri-apps/api` invoke calls
- **Routes**: `/` (dashboard), `/meeting/[id]` (meeting detail), `/settings`

### Backend (`src-tauri/src/`)
- **main.rs**: Minimal entry point, delegates to lib.rs
- **lib.rs**: Tauri command registration and app setup
- **audio.rs**: Audio recording via cpal, saves WAV to app data directory
- **db.rs**: SQLite via sqlx with auto-migrations, tables: `meetings`, `notes`, `templates`
- **whisper.rs**: Local transcription via whisper-rs, model path: `~/.local/share/wizscribe/models/`
- **ai.rs**: OpenAI/Anthropic HTTP clients for summaries and chat

### Data Flow
1. Frontend invokes Tauri command via `api.ts` wrapper
2. Command handler in `lib.rs` calls appropriate module
3. Results return as JSON through Tauri IPC

## Key Patterns

### Adding a New Tauri Command
1. Add function in appropriate Rust module (`audio.rs`, `db.rs`, etc.)
2. Register command in `lib.rs` using `#[tauri::command]` macro and add to `invoke_handler`
3. Add TypeScript wrapper in `src/lib/api.ts`
4. Add types if needed in `src/lib/types/index.ts`

### Frontend State
All global state uses Svelte stores:
```typescript
import { meetings, currentMeeting, isRecording } from '$lib/stores';
```

### Error Handling
- Rust: Uses `anyhow::Result` for error propagation
- Frontend: Tauri commands throw on error, handle with try/catch

## Database Schema

```sql
meetings (id TEXT PK, title, created_at, audio_path, duration_secs, transcript, summary)
notes (id TEXT, meeting_id FK, content, timestamps_json DEFAULT '[]', updated_at)
templates (id TEXT PK, name UNIQUE, structure_json)
```

## Prerequisites

- Rust toolchain via rustup
- Node.js v18+
- System libs: `libwebkit2gtk-4.1-dev libgtk-3-dev libappindicator3-dev librsvg2-dev patchelf`
- Whisper model: Run `./scripts/setup-whisper.sh` or manually place `ggml-base.en.bin` in `~/.local/share/wizscribe/models/`
