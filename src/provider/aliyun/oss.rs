/// Upload a file to Aliyun OSS
pub async fn upload_file(
    access_key_id: &str,
    access_key_secret: &str,
    bucket: &str,
    object_key: &str,
    local_path: &str,
) -> anyhow::Result<()> {
    let data = tokio::fs::read(local_path).await?;
    let url = format!(
        "https://{bucket}.oss-cn-shanghai.aliyuncs.com/{object_key}"
    );

    let date = chrono::Utc::now()
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();

    let string_to_sign = format!(
        "PUT\n\napplication/octet-stream\n{date}\n/{bucket}/{object_key}"
    );

    let signature = {
        use hmac::{Hmac, Mac};
        use sha1::Sha1;
        let mut mac = Hmac::<Sha1>::new_from_slice(access_key_secret.as_bytes())?;
        mac.update(string_to_sign.as_bytes());
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            mac.finalize().into_bytes(),
        )
    };

    let auth = format!("OSS {access_key_id}:{signature}");

    let client = reqwest::Client::new();
    let resp = client
        .put(&url)
        .header("Authorization", auth)
        .header("Date", &date)
        .header("Content-Type", "application/octet-stream")
        .body(data)
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("OSS upload failed: {body}");
    }

    Ok(())
}
