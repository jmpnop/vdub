# KrillinAI

Video dubbing and subtitle engine written in Rust. Automatically transcribes, translates, and dubs videos with support for fully on-device processing on Apple Silicon via MLX. All ASR and TTS providers are free and local.

## Features

- **5-stage pipeline**: Download → Transcribe → Translate → TTS → Embed subtitles
- **Auto language detection**: Whisper detects the source language; EN↔RU translation selected automatically
- **Multi-track audio**: Dubbed audio added as a second track with language metadata (original preserved)
- **yt-dlp integration**: Paste a YouTube URL, get a dubbed `.mp4` back
- **Free ASR backends**: faster-whisper, whisper.cpp, WhisperKit, MLX Whisper
- **Free TTS backends**: Edge TTS, MLX Audio (Kokoro)
- **Any OpenAI-compatible LLM**: OpenAI, DeepSeek, local `mlx_lm.server`
- **On-device Apple Silicon**: MLX Whisper + MLX Audio + local LLM = zero cloud dependencies
- **Web UI**: Built-in browser interface at `http://localhost:8888`
- **Hot-reloadable config**: Update settings via API without restarting

## Requirements

- Rust 1.75+
- ffmpeg / ffprobe
- yt-dlp (for URL downloads)
- One ASR backend installed (faster-whisper, whisper.cpp, WhisperKit, or mlx-whisper)
- One TTS backend installed (edge-tts or mlx-audio)

## Quick Start

```bash
# Clone and build
git clone https://github.com/jmpnop/krillin_rs.git
cd krillin_rs
cargo build --release

# Copy and edit config
cp config/config-example.toml config/config.toml
# Edit config/config.toml — defaults to faster-whisper + edge-tts

# Run
./target/release/krillin_rs
```

Open `http://127.0.0.1:8888` in your browser.

## Apple Silicon (fully local)

Run the entire pipeline on-device with zero cloud dependencies:

```bash
# Install MLX tools
pip install mlx-whisper mlx-audio mlx-lm

# Start local LLM
mlx_lm.server --model mlx-community/Qwen2.5-7B-Instruct-4bit --port 8080
```

Set in `config/config.toml`:

```toml
[llm]
base_url = "http://localhost:8080/v1"
api_key = "not-needed"
model = "mlx-community/Qwen2.5-7B-Instruct-4bit"

[transcribe]
provider = "mlx-whisper"

[tts]
provider = "mlx-audio"
```

## API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/capability/subtitleTask` | POST | Start a dubbing task |
| `/api/capability/subtitleTask?taskId=...` | GET | Get task status |
| `/api/file` | POST | Upload a file |
| `/api/file/*` | GET | Download output files |
| `/api/config` | GET | Get current config |
| `/api/config` | POST | Update config (hot-reload) |

### Start a task

```json
POST /api/capability/subtitleTask
{
  "url": "https://youtube.com/watch?v=...",
  "origin_language": "",
  "target_lang": "",
  "bilingual": 1,
  "translation_subtitle_pos": 2,
  "tts": 1,
  "tts_voice_code": "en-US-AriaNeural",
  "embed_subtitle_video_type": "horizontal",
  "multi_track": true
}
```

| Field | Type | Description |
|-------|------|-------------|
| `url` | string | YouTube URL or path to local video |
| `origin_language` | string | Source language code, empty for auto-detect |
| `target_lang` | string | Target language code, empty for auto-select (EN↔RU) |
| `bilingual` | 0/1 | Show both languages in subtitles |
| `translation_subtitle_pos` | 1/2 | Translation on top (1) or bottom (2) |
| `tts` | 0/1 | Enable text-to-speech dubbing |
| `tts_voice_code` | string | Voice ID (e.g. `en-US-AriaNeural` for Edge TTS, `af_heart` for MLX Audio) |
| `embed_subtitle_video_type` | string | `horizontal`, `vertical`, `all`, or empty to skip |
| `multi_track` | bool | Add dubbed audio as second track (true) or replace original (false) |

## Providers

### Transcription (ASR) — all free, local
| Provider | Config value | Notes |
|----------|-------------|-------|
| faster-whisper | `fasterwhisper` | Local, Python, default |
| whisper.cpp | `whispercpp` | Local, C++ |
| WhisperKit | `whisperkit` | macOS only, CoreML |
| MLX Whisper | `mlx-whisper` | macOS only, Metal GPU |

### Text-to-Speech — all free, local
| Provider | Config value | Notes |
|----------|-------------|-------|
| Edge TTS | `edge-tts` | Free, Microsoft voices, default |
| MLX Audio (Kokoro) | `mlx-audio` | macOS only, 82M params |

### Translation LLM
Any OpenAI-compatible API. Point `llm.base_url` at your provider.

## Project Structure

```
src/
  config/       # TOML config with provider enums
  dto/          # API request/response types
  handler/      # Axum HTTP handlers
  provider/     # ASR, TTS, LLM provider implementations
    openai/     # OpenAI-compatible Chat (for translation LLM)
    local/      # whisper.cpp, WhisperKit, faster-whisper,
                # edge-tts, MLX Whisper, MLX Audio
  service/      # Pipeline steps (split, transcribe, translate, TTS, embed)
  storage/      # Task store, binary path detection
  types/        # Subtitles, ASS headers, prompts, language maps
  util/         # ffmpeg/ffprobe wrappers, text processing, CLI art
static/         # Embedded web UI
config/         # Example configuration
```

## License

MIT
