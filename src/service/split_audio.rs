use std::path::Path;
use std::process::Stdio;

const SAMPLE_RATE: u32 = 3000;
const ENERGY_WINDOW_SECONDS: f64 = 1.5;
const TOLERANCE_SECONDS: f64 = 8.0;
const MIN_SEGMENT_SECONDS: f64 = 20.0;

/// Find optimal split points in the audio based on silence/low-energy moments
pub async fn get_split_points(
    ffmpeg: &str,
    ffprobe: &str,
    audio: &Path,
    segment_minutes: u32,
) -> anyhow::Result<Vec<f64>> {
    let duration = crate::util::audio::get_duration(ffprobe, audio).await?;
    let segment_duration = segment_minutes as f64 * 60.0;

    if duration <= segment_duration {
        return Ok(vec![0.0, duration]);
    }

    let num_segments = (duration / segment_duration).ceil() as usize;
    let mut split_points = vec![0.0];

    for i in 1..num_segments {
        let target = i as f64 * segment_duration;
        let search_start = (target - TOLERANCE_SECONDS).max(split_points.last().copied().unwrap_or(0.0) + MIN_SEGMENT_SECONDS);
        let search_end = (target + TOLERANCE_SECONDS).min(duration);

        if search_start >= search_end {
            split_points.push(target.min(duration));
            continue;
        }

        let quiet_point = get_quietest_point(ffmpeg, audio, search_start, search_end).await?;
        // Ensure minimum segment duration
        if quiet_point - split_points.last().unwrap() >= MIN_SEGMENT_SECONDS {
            split_points.push(quiet_point);
        } else {
            split_points.push(target.min(duration));
        }
    }

    split_points.push(duration);
    Ok(split_points)
}

/// Find the quietest point in a time range using energy analysis
async fn get_quietest_point(
    ffmpeg: &str,
    audio: &Path,
    start: f64,
    end: f64,
) -> anyhow::Result<f64> {
    // Extract raw audio samples with lowpass/highpass filter
    let output = tokio::process::Command::new(ffmpeg)
        .args([
            "-y",
            "-ss", &format!("{start}"),
            "-to", &format!("{end}"),
            "-i", audio.to_str().unwrap(),
            "-f", "s16le",
            "-ar", &SAMPLE_RATE.to_string(),
            "-ac", "1",
            "-af", "lowpass=f=3000,highpass=f=300",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await?;

    let samples = &output.stdout;
    if samples.len() < 4 {
        return Ok((start + end) / 2.0);
    }

    // Convert bytes to i16 samples
    let sample_count = samples.len() / 2;
    let window_samples = (ENERGY_WINDOW_SECONDS * SAMPLE_RATE as f64) as usize;

    if sample_count < window_samples {
        return Ok((start + end) / 2.0);
    }

    // Sliding window energy analysis
    let mut min_energy = f64::MAX;
    let mut min_pos = 0usize;

    let mut current_energy: f64 = 0.0;

    // Initialize first window
    for i in 0..window_samples {
        let sample = i16::from_le_bytes([
            samples[i * 2],
            samples.get(i * 2 + 1).copied().unwrap_or(0),
        ]) as f64;
        current_energy += sample * sample;
    }

    if current_energy < min_energy {
        min_energy = current_energy;
        min_pos = window_samples / 2;
    }

    // Slide window
    for i in window_samples..sample_count {
        let new_sample = i16::from_le_bytes([
            samples[i * 2],
            samples.get(i * 2 + 1).copied().unwrap_or(0),
        ]) as f64;
        let old_sample = i16::from_le_bytes([
            samples[(i - window_samples) * 2],
            samples.get((i - window_samples) * 2 + 1).copied().unwrap_or(0),
        ]) as f64;

        current_energy += new_sample * new_sample - old_sample * old_sample;

        if current_energy < min_energy {
            min_energy = current_energy;
            min_pos = i - window_samples / 2;
        }
    }

    let time_offset = min_pos as f64 / SAMPLE_RATE as f64;
    Ok(start + time_offset)
}

/// Clip a segment from an audio file
pub async fn clip_audio(
    ffmpeg: &str,
    input: &Path,
    output: &Path,
    start: f64,
    end: f64,
) -> anyhow::Result<()> {
    let status = tokio::process::Command::new(ffmpeg)
        .args([
            "-y",
            "-ss", &format!("{start}"),
            "-to", &format!("{end}"),
            "-i", input.to_str().unwrap(),
            output.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("Failed to clip audio segment {start}-{end}");
    }
    Ok(())
}
