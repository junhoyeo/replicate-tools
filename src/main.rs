use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use base64::Engine as _;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::Semaphore;

const API_TOKEN: &str = "r8_D9Vb0uVmeZblQHyuJdbyJItK3l9T51j1slO2Z";
const MAX_CONCURRENT: usize = 10;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: remove-bg <output-dir-name> <model-version-hash>");
        std::process::exit(1);
    }
    let dir_name = &args[1];
    let model_version = &args[2];

    let home = std::env::var("HOME")?;
    let base = PathBuf::from(&home).join("replicate-remove-background");
    let frames_dir = base.join("frames");
    let output_dir = base.join("output").join(dir_name);
    std::fs::create_dir_all(&output_dir)?;

    let mut frames: Vec<PathBuf> = std::fs::read_dir(&frames_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "png"))
        .collect();
    frames.sort();

    let frames: Vec<PathBuf> = frames
        .into_iter()
        .filter(|p| {
            let out = output_dir.join(p.file_name().unwrap());
            !out.exists()
        })
        .collect();

    let total = frames.len();
    if total == 0 {
        println!("[{}] All frames already processed!", dir_name);
        return Ok(());
    }
    println!(
        "[{}] Processing {} frames with {} concurrent workers",
        dir_name, total, MAX_CONCURRENT
    );

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
    let completed = Arc::new(AtomicUsize::new(0));
    let model_version = Arc::new(model_version.to_string());
    let dir_label = Arc::new(dir_name.to_string());

    let mut handles = Vec::new();

    for frame_path in frames {
        let filename = frame_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let output_path = output_dir.join(&filename);
        let client = client.clone();
        let sem = semaphore.clone();
        let completed = completed.clone();
        let version = model_version.clone();
        let label = dir_label.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let image_data = tokio::fs::read(&frame_path).await?;
            let b64 =
                base64::engine::general_purpose::STANDARD.encode(&image_data);
            let data_uri = format!("data:image/png;base64,{}", b64);

            let create_resp = client
                .post("https://api.replicate.com/v1/predictions")
                .header("Authorization", format!("Bearer {}", API_TOKEN))
                .json(&json!({
                    "version": version.as_str(),
                    "input": {
                        "image": data_uri
                    }
                }))
                .send()
                .await?;

            let create_body: Value = create_resp.json().await?;
            let prediction_id = create_body["id"]
                .as_str()
                .ok_or_else(|| {
                    anyhow!(
                        "No prediction ID for {}: {:?}",
                        filename,
                        create_body
                    )
                })?
                .to_string();

            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500))
                    .await;

                let poll_resp = client
                    .get(format!(
                        "https://api.replicate.com/v1/predictions/{}",
                        prediction_id
                    ))
                    .header("Authorization", format!("Bearer {}", API_TOKEN))
                    .send()
                    .await?;

                let poll_body: Value = poll_resp.json().await?;
                let status =
                    poll_body["status"].as_str().unwrap_or("unknown");

                match status {
                    "succeeded" => {
                        let output_url =
                            poll_body["output"].as_str().ok_or_else(|| {
                                anyhow!("No output URL for {}", filename)
                            })?;

                        let img_bytes = client
                            .get(output_url)
                            .send()
                            .await?
                            .bytes()
                            .await?;
                        tokio::fs::write(&output_path, &img_bytes).await?;

                        let done =
                            completed.fetch_add(1, Ordering::Relaxed) + 1;
                        println!(
                            "[{}] [{}/{}] ✓ {}",
                            label, done, total, filename
                        );
                        break;
                    }
                    "failed" | "canceled" => {
                        let error = &poll_body["error"];
                        eprintln!(
                            "[{}] [FAIL] ✗ {} - {}",
                            label, filename, error
                        );
                        break;
                    }
                    _ => continue,
                }
            }

            Ok::<_, anyhow::Error>(())
        });

        handles.push(handle);
    }

    let mut errors = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                eprintln!("[{}] Task error: {}", dir_name, e);
                errors += 1;
            }
            Err(e) => {
                eprintln!("[{}] Join error: {}", dir_name, e);
                errors += 1;
            }
        }
    }

    println!(
        "\n[{}] Done! {}/{} frames processed successfully.",
        dir_name,
        total - errors,
        total
    );
    Ok(())
}
