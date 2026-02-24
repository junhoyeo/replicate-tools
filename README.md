# replicate-tools

Batch background removal and upscaling for video frames using Replicate models, plus frame-to-video joining via FFmpeg. Written in Rust with async concurrency (10 workers).

> [!TIP]
> When using Midjourney, I recommend generating upscaled/HD videos first and then removing the background.

> [!NOTE]
> The assets used in the examples were used in [junhoyeo/tokscale](https://github.com/junhoyeo/tokscale)'s [Landing Page](https://tokscale.ai).

## Prerequisites

- [Rust](https://rustup.rs/) toolchain
- FFmpeg development libraries (libvpx-vp9 for the `join` command)
- [Replicate](https://replicate.com/) API token (set `API_TOKEN` in `src/main.rs`)

## Usage

### Batch Process Frames

```bash
cargo run --release -- <output-dir> <version> [input-dir] [extra-json]
```

| Argument | Description |
|----------|-------------|
| `output-dir` | Name for the output subdirectory (created under `~/replicate-remove-background/output/`) |
| `version` | Replicate model version hash |
| `input-dir` | Input frames directory (default: `frames`) |
| `extra-json` | Extra JSON merged into each prediction input, e.g. `'{"scale":2}'` |
- Skips already-processed frames automatically
- Prints per-frame progress, then a summary with total predict time, wall time, and cost

**Path resolution:** `input-dir` is resolved in order — absolute/relative path → `~/replicate-remove-background/output/<input-dir>` → `~/replicate-remove-background/<input-dir>`.

### Join Frames into Video

Combine processed PNG frames into a transparent WebM video (VP9 + YUVA420P).

```bash
cargo run --release -- join <input-dir> <output.webm> [--fps 24] [--bitrate 4M]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--fps` | `24` | Frame rate |
| `--bitrate` | `4M` | Video bitrate (supports `M`/`K` suffixes, e.g. `8M`, `4000K`) |

- Validates that all frames share the same dimensions before encoding
- `input-dir` also uses smart path resolution (same as batch processing)

## Background Removal — Model Comparison

Slot Machine — Frame 60 of 121 (`frame_0060.png`, 960×960):

| Original | cjwbw/rembg |
|:---:|:---:|
| ![Original](.github/assets/slot-machine-original.png) | ![Output](.github/assets/slot-machine-output.png) |

Frame 60 of 121 (`frame_0060.png`, 560×704):
| Original | lucataco/remove-bg | smoretalk/rembg-enhance | cjwbw/rembg | pollinations/modnet |
|:---:|:---:|:---:|:---:|:---:|
| ![Original](.github/assets/original.png) | ![lucataco](.github/assets/lucataco-remove-bg.png) | ![smoretalk](.github/assets/smoretalk-rembg-enhance.png) | ![cjwbw](.github/assets/cjwbw-rembg.png) | ![modnet](.github/assets/pollinations-modnet.png) |

**Verdict:** **`cjwbw/rembg`** — cleanest shadow removal with best edge preservation.

## 2× Upscaling — Model Comparison

Same frame after `cjwbw/rembg` → 2× upscale (1120×1408):
| cjwbw/rembg (source) | daanelson/real-esrgan-a100 | lucataco/real-esrgan | cjwbw/real-esrgan |
|:---:|:---:|:---:|:---:|
| ![source](.github/assets/cjwbw-rembg.png) | ![daanelson](.github/assets/daanelson-real-esrgan-a100.png) | ![lucataco](.github/assets/lucataco-real-esrgan.png) | ![cjwbw](.github/assets/cjwbw-real-esrgan.png) |

## Models

### Background Removal

| Model | Version | Speed |
|-------|---------|-------|
| `lucataco/remove-bg` | `95fcc2a2...` | Fast |
| `smoretalk/rembg-enhance` | `4067ee2a...` | Slow |
| `cjwbw/rembg` | `fb8af171...` | Medium |
| `pollinations/modnet` | `da7d45f3...` | Fastest |

### Upscaling (2×)

| Model | Version | Speed | Output Size |
|-------|---------|-------|-------------|
| `daanelson/real-esrgan-a100` | `f94d7ed4...` | Fastest | 125M |
| `lucataco/real-esrgan` | `3febd193...` | Medium | 125M |
| `cjwbw/real-esrgan` | `d0ee3d70...` | Slow | 96M |
