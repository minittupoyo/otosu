use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tauri::{Emitter, Manager};
use tauri_plugin_shell::{process::CommandEvent, ShellExt};

const YT_DLP_RELEASE_BASE_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";

#[derive(Serialize, Deserialize, Debug)]
pub struct VideoInfo {
    title: String,
    duration: Option<f64>,
    thumbnail: Option<String>,
    #[serde(rename = "_type")]
    entry_type: Option<String>,
    entries: Option<Vec<PlaylistEntry>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlaylistEntry {
    title: Option<String>,
    duration: Option<f64>,
    thumbnail: Option<String>,
    playlist_index: Option<u32>,
}

#[derive(Clone, Serialize)]
struct DownloadProgress {
    task_id: String,
    progress: f64,
    status: String,
    speed: String,
    eta: String,
    error: Option<String>,
}

#[derive(Serialize)]
struct DependencyStatus {
    installed: bool,
    version: Option<String>,
    message: Option<String>,
}

#[derive(Serialize)]
struct DependencyReport {
    yt_dlp: DependencyStatus,
    ffmpeg: DependencyStatus,
    deno: DependencyStatus,
}

fn yt_dlp_asset_name() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some("yt-dlp.exe"),
        ("windows", "aarch64") => Some("yt-dlp_arm64.exe"),
        ("macos", "x86_64" | "aarch64") => Some("yt-dlp_macos"),
        ("linux", "x86_64") => Some("yt-dlp_linux"),
        ("linux", "aarch64") => Some("yt-dlp_linux_aarch64"),
        _ => None,
    }
}

fn managed_yt_dlp_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let filename = if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };
    app_handle
        .path()
        .app_data_dir()
        .map(|directory| directory.join("tools").join(filename))
        .map_err(|error| format!("アプリデータフォルダーを取得できません: {error}"))
}

async fn executable_version(
    app_handle: &tauri::AppHandle,
    program: impl AsRef<std::ffi::OsStr>,
    argument: &str,
) -> Option<String> {
    let output = app_handle
        .shell()
        .command(program)
        .arg(argument)
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    (!version.is_empty()).then_some(version)
}

async fn resolve_yt_dlp(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let managed_path = managed_yt_dlp_path(app_handle)?;
    if managed_path.is_file()
        && executable_version(app_handle, &managed_path, "--version")
            .await
            .is_some()
    {
        return Ok(managed_path);
    }
    if executable_version(app_handle, "yt-dlp", "--version")
        .await
        .is_some()
    {
        return Ok(PathBuf::from("yt-dlp"));
    }
    Err("yt-dlpがセットアップされていません".to_string())
}

async fn install_yt_dlp(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let asset = yt_dlp_asset_name().ok_or_else(|| {
        format!(
            "この環境には自動セットアップできません: {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;
    let destination = managed_yt_dlp_path(app_handle)?;
    let parent = destination
        .parent()
        .ok_or_else(|| "yt-dlpの保存先が不正です".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("yt-dlpの保存先を作成できません: {error}"))?;

    let url = format!("{YT_DLP_RELEASE_BASE_URL}/{asset}");
    let response = reqwest::get(url)
        .await
        .map_err(|error| format!("yt-dlpのダウンロードに失敗しました: {error}"))?
        .error_for_status()
        .map_err(|error| format!("yt-dlpのダウンロードに失敗しました: {error}"))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("yt-dlpの受信に失敗しました: {error}"))?;
    let temporary = destination.with_extension("download");
    std::fs::write(&temporary, bytes)
        .map_err(|error| format!("yt-dlpを保存できません: {error}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("yt-dlpの実行権限を設定できません: {error}"))?;
    }

    if destination.exists() {
        std::fs::remove_file(&destination)
            .map_err(|error| format!("破損したyt-dlpを削除できません: {error}"))?;
    }
    std::fs::rename(&temporary, &destination)
        .map_err(|error| format!("yt-dlpのセットアップを完了できません: {error}"))?;
    Ok(destination)
}

fn deno_version_is_supported(version: &str) -> bool {
    let Some(number) = version.split_whitespace().nth(1) else {
        return false;
    };
    let mut parts = number.trim_start_matches('v').split('.');
    let major = parts.next().and_then(|part| part.parse::<u32>().ok());
    let minor = parts.next().and_then(|part| part.parse::<u32>().ok());
    matches!((major, minor), (Some(major), Some(minor)) if major > 2 || (major == 2 && minor >= 3))
}

#[tauri::command]
async fn setup_dependencies(app_handle: tauri::AppHandle) -> DependencyReport {
    let yt_dlp_path = match resolve_yt_dlp(&app_handle).await {
        Ok(path) => Ok(path),
        Err(_) => install_yt_dlp(&app_handle).await,
    };
    let yt_dlp = match yt_dlp_path {
        Ok(path) => DependencyStatus {
            installed: true,
            version: executable_version(&app_handle, path, "--version").await,
            message: None,
        },
        Err(error) => DependencyStatus {
            installed: false,
            version: None,
            message: Some(error),
        },
    };

    let ffmpeg_version = executable_version(&app_handle, "ffmpeg", "-version").await;
    let deno_version = executable_version(&app_handle, "deno", "--version").await;
    let deno_supported = deno_version
        .as_deref()
        .is_some_and(deno_version_is_supported);
    DependencyReport {
        yt_dlp,
        ffmpeg: DependencyStatus {
            installed: ffmpeg_version.is_some(),
            version: ffmpeg_version,
            message: None,
        },
        deno: DependencyStatus {
            installed: deno_supported,
            version: deno_version,
            message: (!deno_supported).then(|| "Deno 2.3以上が必要です".to_string()),
        },
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfileInfo {
    name: String,
    path: String,
    is_default: bool,
}

fn parse_progress_line(line: &str) -> Option<(f64, String, String)> {
    if !line.contains("[download]") {
        return None;
    }

    let tokens: Vec<&str> = line.split_whitespace().collect();
    let mut percent = None;
    let mut speed = String::new();
    let mut eta = String::new();

    for (i, token) in tokens.iter().enumerate() {
        if let Some(number) = token.strip_suffix('%') {
            if let Ok(p) = number.parse::<f64>() {
                percent = Some(p);
            }
        }
        if *token == "at" && i + 1 < tokens.len() {
            speed = tokens[i + 1].to_string();
        }
        if *token == "ETA" && i + 1 < tokens.len() {
            eta = tokens[i + 1].to_string();
        }
    }

    percent.map(|p| (p, speed, eta))
}

fn parse_playlist_position(line: &str) -> Option<(u32, u32)> {
    let marker = "Downloading item ";
    let position = line.find(marker)? + marker.len();
    let mut parts = line[position..].split_whitespace();
    let current = parts.next()?.parse().ok()?;
    if parts.next()? != "of" {
        return None;
    }
    let total = parts.next()?.parse().ok()?;
    (current > 0 && total > 0 && current <= total).then_some((current, total))
}

fn aggregate_playlist_progress(percent: f64, position: Option<(u32, u32)>) -> f64 {
    let Some((current, total)) = position else {
        return percent;
    };

    ((f64::from(current - 1) + percent / 100.0) / f64::from(total)) * 100.0
}

fn selected_playlist_position(position: (u32, u32), items: Option<&[u32]>) -> (u32, u32) {
    let Some(items) = items else {
        return position;
    };
    let Some(selected_index) = items.iter().position(|item| *item == position.0) else {
        return position;
    };
    let Ok(current) = u32::try_from(selected_index + 1) else {
        return position;
    };
    let Ok(total) = u32::try_from(items.len()) else {
        return position;
    };

    (current, total)
}

fn normalize_path(path: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let path_str = path.to_string_lossy().replace("/", "\\");
        PathBuf::from(path_str)
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.to_path_buf()
    }
}

fn get_profiles_from_ini(ini_path: &Path, base_dir: &Path) -> Vec<ProfileInfo> {
    let mut profiles = vec![];
    let file = match std::fs::File::open(ini_path) {
        Ok(f) => f,
        Err(_) => return profiles,
    };
    let reader = BufReader::new(file);

    let mut current_name: Option<String> = None;
    let mut current_path: Option<String> = None;
    let mut current_is_relative = true;
    let mut is_default = false;
    let mut inside_profile_section = false;

    for line in reader.lines().map_while(Result::ok) {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            if inside_profile_section {
                if let Some(name) = current_name.take() {
                    if let Some(p) = current_path.take() {
                        let full_path = if current_is_relative {
                            base_dir.join(p)
                        } else {
                            PathBuf::from(p)
                        };
                        let normalized = normalize_path(&full_path);
                        profiles.push(ProfileInfo {
                            name,
                            path: normalized.to_string_lossy().to_string(),
                            is_default,
                        });
                    }
                }
            }
            current_name = None;
            current_path = None;
            current_is_relative = true;
            is_default = false;
            inside_profile_section = line.starts_with("[Profile");
        } else if inside_profile_section {
            if let Some(eq_idx) = line.find('=') {
                let key = line[..eq_idx].trim();
                let val = line[eq_idx + 1..].trim();
                match key {
                    "Name" => current_name = Some(val.to_string()),
                    "Path" => current_path = Some(val.to_string()),
                    "IsRelative" => current_is_relative = val == "1",
                    "Default" => is_default = val == "1",
                    _ => {}
                }
            }
        }
    }

    if inside_profile_section {
        if let Some(name) = current_name {
            if let Some(p) = current_path {
                let full_path = if current_is_relative {
                    base_dir.join(p)
                } else {
                    PathBuf::from(p)
                };
                let normalized = normalize_path(&full_path);
                profiles.push(ProfileInfo {
                    name,
                    path: normalized.to_string_lossy().to_string(),
                    is_default,
                });
            }
        }
    }

    profiles
}

fn get_fallback_profiles(base_dir: &Path) -> Vec<ProfileInfo> {
    let mut profiles = vec![];

    if let Ok(entries) = std::fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("cookies.sqlite").exists() {
                let folder_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                profiles.push(ProfileInfo {
                    name: folder_name,
                    path: normalize_path(&path).to_string_lossy().to_string(),
                    is_default: false,
                });
            }
        }
    }

    let profiles_sub = base_dir.join("Profiles");
    if profiles_sub.exists() && profiles_sub.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&profiles_sub) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("cookies.sqlite").exists() {
                    let folder_name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    profiles.push(ProfileInfo {
                        name: format!("Profiles/{}", folder_name),
                        path: normalize_path(&path).to_string_lossy().to_string(),
                        is_default: false,
                    });
                }
            }
        }
    }

    profiles
}

#[tauri::command]
fn get_browser_profiles(browser: String) -> Result<Vec<ProfileInfo>, String> {
    let home = std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from));
    let _appdata = std::env::var("APPDATA").ok().map(PathBuf::from);

    let home = match home {
        Some(h) => h,
        None => return Ok(vec![]),
    };

    let mut search_dirs = vec![];

    match browser.to_lowercase().as_str() {
        "firefox" => {
            #[cfg(target_os = "windows")]
            {
                if let Some(ref ad) = _appdata {
                    search_dirs.push(ad.join("Mozilla").join("Firefox"));
                }
            }
            #[cfg(target_os = "macos")]
            {
                search_dirs.push(
                    home.join("Library")
                        .join("Application Support")
                        .join("Firefox"),
                );
            }
            #[cfg(target_os = "linux")]
            {
                search_dirs.push(home.join(".mozilla").join("firefox"));
                search_dirs.push(
                    home.join(".var")
                        .join("app")
                        .join("org.mozilla.firefox")
                        .join(".mozilla")
                        .join("firefox"),
                );
                search_dirs.push(
                    home.join("snap")
                        .join("firefox")
                        .join("common")
                        .join(".mozilla")
                        .join("firefox"),
                );
            }
        }
        "floorp" => {
            #[cfg(target_os = "windows")]
            {
                if let Some(ref ad) = _appdata {
                    search_dirs.push(ad.join("Floorp"));
                }
            }
            #[cfg(target_os = "macos")]
            {
                search_dirs.push(
                    home.join("Library")
                        .join("Application Support")
                        .join("Floorp"),
                );
            }
            #[cfg(target_os = "linux")]
            {
                search_dirs.push(home.join(".floorp"));
                search_dirs.push(
                    home.join(".var")
                        .join("app")
                        .join("one.ablaze.floorp")
                        .join(".floorp"),
                );
            }
        }
        "zen" => {
            #[cfg(target_os = "windows")]
            {
                if let Some(ref ad) = _appdata {
                    search_dirs.push(ad.join("zen"));
                    search_dirs.push(ad.join("Zen"));
                }
            }
            #[cfg(target_os = "macos")]
            {
                search_dirs.push(home.join("Library").join("Application Support").join("zen"));
                search_dirs.push(home.join("Library").join("Application Support").join("Zen"));
            }
            #[cfg(target_os = "linux")]
            {
                search_dirs.push(home.join(".zen"));
                search_dirs.push(
                    home.join(".var")
                        .join("app")
                        .join("io.github.zen_browser.zen")
                        .join(".zen"),
                );
            }
        }
        _ => return Ok(vec![]),
    }

    let mut all_profiles = vec![];

    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }

        let profiles_ini_path = dir.join("profiles.ini");
        let mut profiles = vec![];
        if profiles_ini_path.exists() {
            profiles = get_profiles_from_ini(&profiles_ini_path, &dir);
        }

        if profiles.is_empty() {
            profiles = get_fallback_profiles(&dir);
        }

        all_profiles.extend(profiles);
    }

    let mut unique_profiles = vec![];
    let mut seen_paths = std::collections::HashSet::new();
    for p in all_profiles {
        if !seen_paths.contains(&p.path) {
            seen_paths.insert(p.path.clone());
            unique_profiles.push(p);
        }
    }

    Ok(unique_profiles)
}

#[tauri::command]
async fn fetch_video_info(app_handle: tauri::AppHandle, url: String) -> Result<VideoInfo, String> {
    let yt_dlp = resolve_yt_dlp(&app_handle).await?;
    let output = app_handle
        .shell()
        .command(yt_dlp)
        .arg("--dump-single-json")
        .arg("--flat-playlist")
        .arg(&url)
        .output()
        .await
        .map_err(|e| format!("エラーが発生しました: {}", e))?;
    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("エラー: {}", error_msg));
    }
    let json_str = String::from_utf8_lossy(&output.stdout);

    let info: VideoInfo =
        serde_json::from_str(&json_str).map_err(|e| format!("JSONのパースに失敗: {}", e))?;

    Ok(info)
}

#[tauri::command]
#[expect(
    clippy::too_many_arguments,
    reason = "Tauri deserializes these command parameters by their frontend names"
)]
async fn start_download(
    app_handle: tauri::AppHandle,
    task_id: String,
    url: String,
    format: String,
    quality: String,
    embed_subtitles: bool,
    embed_thumbnail: bool,
    embed_metadata: bool,
    playlist_behavior: String,
    write_auto_subs: bool,
    sub_langs: String,
    cookie_profile_path: String,
    download_dir: String,
    playlist_items: Option<Vec<u32>>,
) -> Result<(), String> {
    if playlist_behavior == "yes"
        && playlist_items
            .as_ref()
            .is_some_and(|items| items.is_empty() || items.contains(&0))
    {
        return Err("ダウンロードする動画を1本以上選択してください".to_string());
    }

    tauri::async_runtime::spawn(async move {
        let yt_dlp = match resolve_yt_dlp(&app_handle).await {
            Ok(path) => path,
            Err(error) => {
                let _ = app_handle.emit(
                    "download-progress",
                    DownloadProgress {
                        task_id: task_id.clone(),
                        progress: 0.0,
                        status: "failed".to_string(),
                        speed: String::new(),
                        eta: String::new(),
                        error: Some(error),
                    },
                );
                return;
            }
        };
        let mut args = vec![];

        // Output directory
        let target_dir = if download_dir == "default" {
            app_handle
                .path()
                .download_dir()
                .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default())
        } else {
            PathBuf::from(download_dir)
        };
        let output_template = if playlist_behavior == "yes" {
            target_dir.join("%(playlist).60B/%(playlist_index)03d - %(title).120B.%(ext)s")
        } else {
            target_dir.join("%(title).180B.%(ext)s")
        };
        let output_str = output_template.to_string_lossy().to_string();
        args.push("-o".to_string());
        args.push(output_str);

        args.push("--newline".to_string());

        #[cfg(target_os = "windows")]
        args.push("--windows-filenames".to_string());

        // Format and Quality options
        if format == "mp4" || format == "mkv" {
            let f_arg = match quality.as_str() {
                "2160p" => "bestvideo[height<=2160]+bestaudio/best[height<=2160]",
                "1440p" => "bestvideo[height<=1440]+bestaudio/best[height<=1440]",
                "1080p" => "bestvideo[height<=1080]+bestaudio/best[height<=1080]",
                "720p" => "bestvideo[height<=720]+bestaudio/best[height<=720]",
                _ => "bestvideo+bestaudio/best", // best / auto
            };
            args.push("-f".to_string());
            args.push(f_arg.to_string());
            args.push("--merge-output-format".to_string());
            args.push(format.clone());
        } else {
            args.push("-x".to_string());
            args.push("--audio-format".to_string());
            args.push(format.clone());

            let audio_q = match format.as_str() {
                "mp3" => match quality.as_str() {
                    "320k" => "320k",
                    "256k" => "256k",
                    "192k" => "192k",
                    "128k" => "128k",
                    _ => "0", // best / auto
                },
                _ => "0", // flac, wav -> auto (best)
            };
            args.push("--audio-quality".to_string());
            args.push(audio_q.to_string());
        }

        // Subtitles configuration
        if embed_subtitles {
            args.push("--embed-subs".to_string());
            let lang_filter = match sub_langs.as_str() {
                "ja" => "ja",
                "en" => "en",
                _ => "all",
            };
            args.push("--sub-langs".to_string());
            args.push(lang_filter.to_string());

            if write_auto_subs {
                args.push("--write-auto-subs".to_string());
            }
        }

        // Embed thumbnail
        if embed_thumbnail {
            args.push("--embed-thumbnail".to_string());
        }

        // Embed metadata
        if embed_metadata {
            args.push("--embed-metadata".to_string());
        }

        // Playlist behavior
        if playlist_behavior == "yes" {
            args.push("--yes-playlist".to_string());
            if let Some(items) = playlist_items.as_ref() {
                args.push("--playlist-items".to_string());
                args.push(
                    items
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
        } else {
            args.push("--no-playlist".to_string());
        }

        // Cookie source option
        if cookie_profile_path != "none" {
            if Path::new(&cookie_profile_path).exists() {
                let cookie_arg = format!("firefox:{}", cookie_profile_path);
                args.push("--cookies-from-browser".to_string());
                args.push(cookie_arg);
            } else {
                let _ = app_handle.emit(
                    "download-progress",
                    DownloadProgress {
                        task_id: task_id.clone(),
                        progress: 0.0,
                        status: "failed".to_string(),
                        speed: String::new(),
                        eta: String::new(),
                        error: Some(format!(
                            "指定された Cookie プロファイルパスが存在しません: {}",
                            cookie_profile_path
                        )),
                    },
                );
                return;
            }
        }

        args.push(url);

        let (mut receiver, _child) = match app_handle.shell().command(yt_dlp).args(&args).spawn() {
            Ok(process) => process,
            Err(e) => {
                let _ = app_handle.emit(
                    "download-progress",
                    DownloadProgress {
                        task_id: task_id.clone(),
                        progress: 0.0,
                        status: "failed".to_string(),
                        speed: String::new(),
                        eta: String::new(),
                        error: Some(format!("yt-dlpの起動に失敗しました: {}", e)),
                    },
                );
                return;
            }
        };

        let mut playlist_position = None;
        let mut exit_code = None;
        let mut err_msg = String::new();

        while let Some(event) = receiver.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let line_str = String::from_utf8_lossy(&line);
                    if let Some(position) = parse_playlist_position(&line_str) {
                        playlist_position = Some(selected_playlist_position(
                            position,
                            playlist_items.as_deref(),
                        ));
                    }
                    if let Some((percent, speed, eta)) = parse_progress_line(&line_str) {
                        let _ = app_handle.emit(
                            "download-progress",
                            DownloadProgress {
                                task_id: task_id.clone(),
                                progress: aggregate_playlist_progress(percent, playlist_position),
                                status: "downloading".to_string(),
                                speed,
                                eta,
                                error: None,
                            },
                        );
                    }
                }
                CommandEvent::Stderr(line) => {
                    err_msg.push_str(&String::from_utf8_lossy(&line));
                    err_msg.push('\n');
                }
                CommandEvent::Error(error) => {
                    err_msg.push_str(&error);
                    err_msg.push('\n');
                }
                CommandEvent::Terminated(payload) => exit_code = payload.code,
                _ => {}
            }
        }

        if exit_code == Some(0) {
            let _ = app_handle.emit(
                "download-progress",
                DownloadProgress {
                    task_id: task_id.clone(),
                    progress: 100.0,
                    status: "done".to_string(),
                    speed: String::new(),
                    eta: String::new(),
                    error: None,
                },
            );
        } else {
            let _ = app_handle.emit(
                "download-progress",
                DownloadProgress {
                    task_id: task_id.clone(),
                    progress: 0.0,
                    status: "failed".to_string(),
                    speed: String::new(),
                    eta: String::new(),
                    error: Some(if err_msg.is_empty() {
                        "ダウンロード中にエラーが発生しました".to_string()
                    } else {
                        err_msg
                    }),
                },
            );
        }
    });

    Ok(())
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {}));

    builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link().register_all()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            setup_dependencies,
            fetch_video_info,
            start_download,
            get_browser_profiles
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{aggregate_playlist_progress, parse_playlist_position, selected_playlist_position};

    #[test]
    fn parse_playlist_position_returns_current_item_and_total() {
        let line = "[download] Downloading item 3 of 12";

        assert_eq!(parse_playlist_position(line), Some((3, 12)));
    }

    #[test]
    fn aggregate_playlist_progress_combines_item_and_file_progress() {
        let progress = aggregate_playlist_progress(50.0, Some((2, 4)));

        assert_eq!(progress, 37.5);
    }

    #[test]
    fn aggregate_playlist_progress_preserves_single_video_progress() {
        let progress = aggregate_playlist_progress(42.0, None);

        assert_eq!(progress, 42.0);
    }

    #[test]
    fn selected_playlist_position_uses_selection_order() {
        let position = selected_playlist_position((7, 10), Some(&[2, 7, 9]));

        assert_eq!(position, (2, 3));
    }

    #[test]
    fn deno_version_is_supported_accepts_minimum_version() {
        assert!(super::deno_version_is_supported("deno 2.3.0"));
    }

    #[test]
    fn deno_version_is_supported_rejects_old_version() {
        assert!(!super::deno_version_is_supported("deno 2.2.9"));
    }
}
