//! Dependency management — auto-detect and install required tools
//!
//! Homebrew tools: ffmpeg, yt-dlp, whisper-cpp, whisperkit-cli
//! Python venv tools: faster-whisper, mlx-whisper, edge-tts, mlx-audio, mlx-lm

use crate::config::{Config, TranscribeProvider, TtsProvider};
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// Where the managed Python venv lives
const VENV_DIR: &str = "./venv";

/// A dependency that may need to be installed
struct Dep {
    name: &'static str,
    kind: DepKind,
    check: Check,
}

enum DepKind {
    Brew(&'static str),      // brew formula name
    Pip(&'static str),       // pip package name
}

enum Check {
    Binary(&'static str),    // check `which <binary>`
    PipPkg(&'static str),    // check `pip show <pkg>` in venv
}

/// Ensure all dependencies for the current config are installed.
/// Returns the venv bin directory path (if a venv was created).
pub async fn ensure_dependencies(config: &Config) -> anyhow::Result<Option<PathBuf>> {
    let mut deps: Vec<Dep> = Vec::new();

    // Always required
    deps.push(Dep {
        name: "ffmpeg",
        kind: DepKind::Brew("ffmpeg"),
        check: Check::Binary("ffmpeg"),
    });
    deps.push(Dep {
        name: "yt-dlp",
        kind: DepKind::Brew("yt-dlp"),
        check: Check::Binary("yt-dlp"),
    });

    // ASR provider
    match config.transcribe.provider {
        TranscribeProvider::Fasterwhisper => {
            deps.push(Dep {
                name: "faster-whisper",
                kind: DepKind::Pip("faster-whisper"),
                check: Check::PipPkg("faster-whisper"),
            });
        }
        TranscribeProvider::Whispercpp => {
            deps.push(Dep {
                name: "whisper-cpp",
                kind: DepKind::Brew("whisper-cpp"),
                check: Check::Binary("whisper-cpp"),
            });
        }
        TranscribeProvider::Whisperkit => {
            deps.push(Dep {
                name: "whisperkit-cli",
                kind: DepKind::Brew("whisperkit-cli"),
                check: Check::Binary("whisperkit-cli"),
            });
        }
        TranscribeProvider::MlxWhisper => {
            deps.push(Dep {
                name: "mlx-whisper",
                kind: DepKind::Pip("mlx-whisper"),
                check: Check::PipPkg("mlx-whisper"),
            });
        }
    }

    // TTS provider
    match config.tts.provider {
        TtsProvider::EdgeTts => {
            deps.push(Dep {
                name: "edge-tts",
                kind: DepKind::Pip("edge-tts"),
                check: Check::PipPkg("edge-tts"),
            });
        }
        TtsProvider::MlxAudio => {
            deps.push(Dep {
                name: "mlx-audio",
                kind: DepKind::Pip("mlx-audio"),
                check: Check::PipPkg("mlx-audio"),
            });
        }
    }

    // Check which deps are missing
    let needs_venv = deps.iter().any(|d| matches!(d.kind, DepKind::Pip(_)));
    let venv_bin = PathBuf::from(VENV_DIR).join("bin");
    let pip_path = venv_bin.join("pip");

    let mut missing_brew: Vec<&Dep> = Vec::new();
    let mut missing_pip: Vec<&Dep> = Vec::new();

    for dep in &deps {
        let installed = match &dep.check {
            Check::Binary(bin) => is_binary_available(bin).await,
            Check::PipPkg(pkg) => {
                if venv_bin.exists() {
                    is_pip_installed(&pip_path, pkg).await
                } else {
                    false
                }
            }
        };

        if installed {
            tracing::info!("   ✅ {} — installed", dep.name);
        } else {
            match &dep.kind {
                DepKind::Brew(_) => missing_brew.push(dep),
                DepKind::Pip(_) => missing_pip.push(dep),
            }
        }
    }

    if missing_brew.is_empty() && missing_pip.is_empty() {
        tracing::info!("   📦 All dependencies satisfied");
        if needs_venv && venv_bin.exists() {
            return Ok(Some(venv_bin));
        }
        return Ok(None);
    }

    // Install missing brew packages
    if !missing_brew.is_empty() {
        ensure_homebrew().await?;
        for dep in &missing_brew {
            if let DepKind::Brew(formula) = &dep.kind {
                tracing::info!("   📥 Installing {} via Homebrew...", dep.name);
                let output = tokio::process::Command::new("brew")
                    .args(["install", formula])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await?;
                if output.status.success() {
                    tracing::info!("   ✅ {} installed", dep.name);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    tracing::error!("   ❌ Failed to install {}: {}", dep.name, stderr.lines().last().unwrap_or(&stderr));
                }
            }
        }
    }

    // Install missing pip packages
    if !missing_pip.is_empty() {
        // Ensure venv exists
        if !Path::new(VENV_DIR).exists() {
            tracing::info!("   🐍 Creating Python virtual environment at {VENV_DIR}/...");
            let output = tokio::process::Command::new("python3")
                .args(["-m", "venv", VENV_DIR])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Failed to create venv: {stderr}");
            }
            tracing::info!("   ✅ Virtual environment created");
        }

        // Upgrade pip first
        let _ = tokio::process::Command::new(pip_path.to_str().unwrap())
            .args(["install", "--upgrade", "pip"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .await;

        // Collect all pip packages to install in one go
        let packages: Vec<&str> = missing_pip
            .iter()
            .filter_map(|d| match &d.kind {
                DepKind::Pip(pkg) => Some(*pkg),
                _ => None,
            })
            .collect();

        let names: Vec<&str> = missing_pip.iter().map(|d| d.name).collect();
        tracing::info!("   📥 Installing pip packages: {}...", names.join(", "));

        let mut args = vec!["install"];
        args.extend(&packages);

        let output = tokio::process::Command::new(pip_path.to_str().unwrap())
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if output.status.success() {
            for name in &names {
                tracing::info!("   ✅ {} installed", name);
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("   ❌ pip install failed: {}", stderr.lines().last().unwrap_or(&stderr));
            // Try installing one by one to see which ones fail
            for dep in &missing_pip {
                if let DepKind::Pip(pkg) = &dep.kind {
                    let output = tokio::process::Command::new(pip_path.to_str().unwrap())
                        .args(["install", pkg])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .output()
                        .await?;
                    if output.status.success() {
                        tracing::info!("   ✅ {} installed", dep.name);
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        tracing::error!("   ❌ Failed to install {}: {}", dep.name, stderr.lines().last().unwrap_or(&stderr));
                    }
                }
            }
        }
    }

    if needs_venv && venv_bin.exists() {
        Ok(Some(venv_bin))
    } else {
        Ok(None)
    }
}

async fn ensure_homebrew() -> anyhow::Result<()> {
    if is_binary_available("brew").await {
        return Ok(());
    }
    tracing::error!("   ❌ Homebrew not found. Install it from https://brew.sh");
    tracing::error!("      /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"");
    anyhow::bail!("Homebrew is required but not installed");
}

async fn is_binary_available(name: &str) -> bool {
    tokio::process::Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn is_pip_installed(pip: &Path, package: &str) -> bool {
    if !pip.exists() {
        return false;
    }
    tokio::process::Command::new(pip)
        .args(["show", package])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}
