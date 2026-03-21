use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use std::process::Stdio;

/// Run an external command, capturing stdout. Returns stdout bytes on success.
/// On failure, reports the command name and stderr.
pub async fn run_cmd(program: &str, args: &[&str]) -> anyhow::Result<Vec<u8>> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{program}: {stderr}");
    }

    Ok(output.stdout)
}

/// Run an external command, discarding stdout. Returns Ok on success.
/// On failure, reports the command name and stderr.
pub async fn run_cmd_status(program: &str, args: &[&str]) -> anyhow::Result<()> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{program}: {stderr}");
    }

    Ok(())
}

/// Run an external command, streaming stderr and calling `on_line` for each line.
/// Used to parse progress output from tools like yt-dlp.
pub async fn run_cmd_with_progress<F>(
    program: &str,
    args: &[&str],
    mut on_line: F,
) -> anyhow::Result<()>
where
    F: FnMut(&str),
{
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().unwrap();
    let mut reader = BufReader::new(stderr).lines();

    while let Some(line) = reader.next_line().await? {
        on_line(&line);
    }

    let status = child.wait().await?;
    if !status.success() {
        anyhow::bail!("{program} exited with {status}");
    }

    Ok(())
}

/// Run an external command with piped stdout for raw data extraction (e.g., audio samples).
pub async fn run_cmd_raw(program: &str, args: &[&str]) -> anyhow::Result<Vec<u8>> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await?;

    Ok(output.stdout)
}
