#!/bin/bash

# WizScribe Whisper Model Setup Script
# This script downloads a Whisper model for WizScribe

set -e

MODEL_DIR="$HOME/.local/share/wizscribe/models"
MODEL_NAME="${1:-ggml-base.en.bin}"
MODEL_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$MODEL_NAME"

echo "Setting up Whisper model for WizScribe..."
echo "Model: $MODEL_NAME"
echo "Destination: $MODEL_DIR"

# Create directory
mkdir -p "$MODEL_DIR"

# Check if model already exists
if [ -f "$MODEL_DIR/$MODEL_NAME" ]; then
    echo "Model already exists at $MODEL_DIR/$MODEL_NAME"
    exit 0
fi

# Download model
echo "Downloading model..."
curl -L --progress-bar "$MODEL_URL" -o "$MODEL_DIR/$MODEL_NAME"

echo ""
echo "Model downloaded successfully!"
echo "Location: $MODEL_DIR/$MODEL_NAME"
echo ""
echo "You can now use WizScribe for transcription."
