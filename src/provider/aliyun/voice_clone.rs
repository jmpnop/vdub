use chrono::Utc;
use hmac::{Hmac, Mac};
use sha1::Sha1;

/// Clone a voice using Aliyun CosyVoice
pub async fn cosy_voice_clone(
    access_key_id: &str,
    access_key_secret: &str,
    voice_prefix: &str,
    audio_url: &str,
) -> anyhow::Result<String> {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let nonce = uuid::Uuid::new_v4().to_string();

    let mut params = vec![
        ("AccessKeyId", access_key_id.to_string()),
        ("Action", "CosyVoiceClone".to_string()),
        ("Format", "JSON".to_string()),
        ("RegionId", "cn-shanghai".to_string()),
        ("SignatureMethod", "HMAC-SHA1".to_string()),
        ("SignatureNonce", nonce),
        ("SignatureVersion", "1.0".to_string()),
        ("Timestamp", timestamp),
        ("Url", audio_url.to_string()),
        ("Version", "2019-08-19".to_string()),
        ("VoicePrefix", voice_prefix.to_string()),
    ];

    params.sort_by(|a, b| a.0.cmp(b.0));

    let query: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let string_to_sign = format!("POST&%2F&{}", percent_encode(&query));
    let sign_key = format!("{access_key_secret}&");

    let mut mac = Hmac::<Sha1>::new_from_slice(sign_key.as_bytes())?;
    mac.update(string_to_sign.as_bytes());
    let signature = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        mac.finalize().into_bytes(),
    );

    let url = format!(
        "https://nls-slp.cn-shanghai.aliyuncs.com/?Signature={}&{}",
        percent_encode(&signature),
        query
    );

    let client = reqwest::Client::new();
    let resp: serde_json::Value = client.post(&url).send().await?.json().await?;

    if resp["Code"].as_i64() != Some(0) && resp["Message"].as_str() != Some("SUCCESS") {
        anyhow::bail!("Voice clone failed: {resp}");
    }

    resp["VoiceName"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No VoiceName in response: {resp}"))
}

fn percent_encode(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}
