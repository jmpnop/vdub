use futures_util::StreamExt;
use std::path::Path;
use tokio::io::AsyncWriteExt;

/// Download a file from URL with optional proxy
pub async fn download_file(url: &str, dest: &Path, proxy: Option<&str>) -> anyhow::Result<()> {
    let mut builder = reqwest::Client::builder();
    if let Some(proxy_url) = proxy {
        if !proxy_url.is_empty() {
            builder = builder.proxy(reqwest::Proxy::all(proxy_url)?);
        }
    }
    let client = builder.build()?;

    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Download failed with status {}", resp.status());
    }

    let total = resp.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        file.write_all(&bytes).await?;
        downloaded += bytes.len() as u64;

        if total > 0 {
            let pct = (downloaded as f64 / total as f64 * 100.0) as u32;
            if pct.is_multiple_of(10) {
                tracing::debug!("Download progress: {pct}%");
            }
        }
    }

    file.flush().await?;
    tracing::info!(
        "Downloaded {} ({} bytes)",
        dest.display(),
        downloaded
    );
    Ok(())
}
