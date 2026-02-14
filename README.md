# replicate-remove-background

Batch background removal for 121 video frames using 4 different Replicate models. Written in Rust with async concurrency (10 workers).

## Usage

```bash
cargo run --release <output-dir-name> <model-version-hash>
```

Skips already-processed frames automatically.

## Model Comparison

Frame 60 of 121 (`frame_0060.png`):

| Original | lucataco/remove-bg | smoretalk/rembg-enhance | cjwbw/rembg | pollinations/modnet |
|:---:|:---:|:---:|:---:|:---:|
| ![Original](.github/assets/original.png) | ![lucataco](.github/assets/lucataco-remove-bg.png) | ![smoretalk](.github/assets/smoretalk-rembg-enhance.png) | ![cjwbw](.github/assets/cjwbw-rembg.png) | ![modnet](.github/assets/pollinations-modnet.png) |

### Verdict

**`cjwbw/rembg`** — cleanest shadow removal with best edge preservation.

## Models

| Model | Version | Speed |
|-------|---------|-------|
| `lucataco/remove-bg` | `95fcc2a2...` | Fast |
| `smoretalk/rembg-enhance` | `4067ee2a...` | Slow |
| `cjwbw/rembg` | `fb8af171...` | Medium |
| `pollinations/modnet` | `da7d45f3...` | Fastest |
