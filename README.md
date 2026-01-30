# WizScribe - AI Meeting Notes App

WizScribe is a powerful AI-powered meeting notes application for Linux, built with Tauri and SvelteKit. It records audio (microphone + system), transcribes using Whisper.cpp, and provides AI-generated summaries and chat capabilities.

## Features

- **Audio Recording**: Capture microphone audio using cpal
- **AI Transcription**: Local transcription using Whisper.cpp (no cloud required)
- **Rich Text Notes**: Take notes with timestamps and formatting
- **AI Summaries**: Generate meeting summaries using OpenAI or Anthropic
- **AI Chat**: Ask questions about your meeting transcripts
- **Templates**: Pre-defined note templates for different meeting types
- **Export**: Export meetings to Markdown format

## Quick Start

### 1. Install System Dependencies

```bash
# Debian/Ubuntu
sudo apt-get update && sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf
```

### 2. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 3. Install Node.js (v18+)

```bash
# Using nvm (recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18

# Or using apt (may be older version)
sudo apt-get install nodejs npm
```

### 4. Clone and Install

```bash
git clone git@github.com:aman-2709/WizScribe.git
cd WizScribe
npm install
```

### 5. Setup Whisper Model

```bash
# Run the setup script (downloads ~150MB model)
./scripts/setup-whisper.sh

# Or manually download:
mkdir -p ~/.local/share/wizscribe/models
curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin \
  -o ~/.local/share/wizscribe/models/ggml-base.en.bin
```

### 6. Run the App

```bash
npm run tauri dev
```

## Adding Your API Key

To use AI features (summaries, chat, action item extraction), you need an API key from OpenAI or Anthropic:

### Getting an API Key

**OpenAI:**
1. Go to [platform.openai.com](https://platform.openai.com)
2. Sign up or log in
3. Navigate to API Keys section
4. Create a new secret key

**Anthropic:**
1. Go to [console.anthropic.com](https://console.anthropic.com)
2. Sign up or log in
3. Navigate to API Keys
4. Create a new key

### Configuring in WizScribe

1. Launch WizScribe
2. Click the **Settings** icon (gear) in the sidebar
3. Select your AI provider (OpenAI or Anthropic)
4. Paste your API key in the input field
5. Click **Save**

Your API key is stored locally at `~/.local/share/wizscribe/config.json` and is never sent anywhere except directly to your chosen AI provider's API.

## Building for Production

```bash
npm run tauri build
```

The built application will be in `src-tauri/target/release/`.

## Project Structure

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
│       ├── main.rs         # Entry point
│       ├── lib.rs          # Tauri commands
│       ├── audio.rs        # Audio recording (cpal)
│       ├── db.rs           # SQLite database
│       ├── whisper.rs      # Whisper transcription
│       ├── config.rs       # App configuration
│       └── ai.rs           # AI integration
├── scripts/
│   └── setup-whisper.sh    # Whisper model download script
└── ...
```

## Tech Stack

- **Frontend**: SvelteKit, TypeScript, Tailwind CSS v4, Lucide icons
- **Backend**: Rust, Tauri 2
- **Database**: SQLite with sqlx
- **Audio**: cpal, hound, rubato
- **Transcription**: whisper.cpp via whisper-rs
- **AI**: OpenAI GPT-4 / Anthropic Claude 3

## Troubleshooting

### "cargo not found" after installing Rust

Run `source "$HOME/.cargo/env"` or restart your terminal.

### Whisper model not found

Make sure the model file exists at `~/.local/share/wizscribe/models/ggml-base.en.bin`. Run the setup script again if needed.

### WebKit errors on Ubuntu

Ensure you have the correct WebKit version:
```bash
sudo apt-get install libwebkit2gtk-4.1-dev
```

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
