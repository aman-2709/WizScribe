# WizScribe - AI Meeting Notes App

WizScribe is a powerful AI-powered meeting notes application for Linux, built with Tauri and SvelteKit. It records audio (microphone + system), transcribes using Whisper.cpp, and provides AI-generated summaries and chat capabilities.

## Features

- **Audio Recording**: Capture both microphone and system audio using PipeWire on Linux
- **AI Transcription**: Local transcription using Whisper.cpp
- **Rich Text Notes**: Take notes with timestamps and formatting
- **AI Summaries**: Generate meeting summaries using OpenAI or Anthropic
- **AI Chat**: Ask questions about your meeting transcripts
- **Templates**: Pre-defined note templates for different meeting types
- **Export**: Export meetings to Markdown format

## Prerequisites

### System Dependencies

```bash
# Debian/Ubuntu
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev libappindicator3-dev librsvg2-dev patchelf

# Fedora
sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel librsvg2-devel patchelf

# Arch Linux
sudo pacman -S webkit2gtk-4.1 gtk3 libappindicator-gtk3 librsvg patchelf
```

### Whisper Model

Download a Whisper model (e.g., ggml-base.en.bin) and place it in:
```
~/.local/share/wizscribe/models/
```

Or set the path in the app settings.

### Rust & Node.js

- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/) (v18+)

## Installation

```bash
# Clone the repository
git clone <repository-url>
cd WizScribe

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Configuration

### AI Provider

1. Go to Settings in the app
2. Choose your AI provider (OpenAI or Anthropic)
3. Enter your API key

Your API key is stored locally and never sent to any servers except the AI provider's API.

### Templates

Create custom note templates in Settings. Templates use JSON structure:
```json
[
  {"type": "heading", "content": "Agenda"},
  {"type": "bullet", "content": ""},
  {"type": "checkbox", "content": "Action item"}
]
```

## Development

### Project Structure

```
WizScribe/
├── src/                    # SvelteKit frontend
│   ├── lib/
│   │   ├── components/     # Svelte components
│   │   ├── stores/         # Svelte stores
│   │   └── types/          # TypeScript types
│   ├── routes/             # SvelteKit routes
│   └── app.css             # Tailwind CSS
├── src-tauri/              # Rust backend
│   └── src/
│       ├── main.rs         # Tauri commands
│       ├── audio.rs        # Audio recording (PipeWire)
│       ├── db.rs           # SQLite database
│       ├── whisper.rs      # Whisper transcription
│       └── ai.rs           # AI integration
└── ...
```

### Key Technologies

- **Frontend**: SvelteKit, TailwindCSS, Lucide icons
- **Backend**: Rust, Tauri
- **Database**: SQLite with sqlx
- **Audio**: PipeWire
- **Transcription**: whisper.cpp via whisper-rs
- **AI**: OpenAI GPT-4 / Anthropic Claude

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
