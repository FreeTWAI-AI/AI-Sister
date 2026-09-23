//! 四支既有 CLI 的偵測、官方登入與實際 bridge 測試。
//!
//! 所有長工作都由 Tauri async command 放進 blocking worker；設定只在固定 token
//! 從真正 bridge 回來後寫入。登入／測試／取消的任何失敗都不會換掉原設定。

use serde::Serialize;
use sister_core::provider_cli::{
    BrainProvider, READY_PROMPT, bridge_args, output_has_ready_token, parse_bridge_args,
};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

const VERSION_TIMEOUT: Duration = Duration::from_secs(5);
const LOGIN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const PROBE_TIMEOUT: Duration = Duration::from_secs(120);
const JOB_IDLE: u8 = 0;
const JOB_ACTIVE: u8 = 1;
const JOB_CANCELLING: u8 = 2;

#[derive(Debug, Clone, Serialize)]
pub struct BrainCliView {
    providers: Vec<ProviderView>,
    selected: Option<&'static str>,
    custom_configured: bool,
    busy: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderView {
    id: &'static str,
    label: &'static str,
    installed: bool,
    version: Option<String>,
    selected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrainCliOutcome {
    pub provider: &'static str,
    pub label: &'static str,
    pub brain: BrainCliView,
}

#[derive(Debug)]
struct RunOutput {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    cancelled: bool,
    timed_out: bool,
}

pub(crate) struct BusyClaim(Arc<AtomicU8>);

impl BusyClaim {
    fn take(slot: &Arc<AtomicU8>) -> Result<Self, String> {
        slot.compare_exchange(JOB_IDLE, JOB_ACTIVE, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "另一個 CLI 登入或測試正在進行".to_owned())?;
        Ok(Self(Arc::clone(slot)))
    }
}

impl Drop for BusyClaim {
    fn drop(&mut self) {
        self.0.store(JOB_IDLE, Ordering::Release);
    }
}

pub(crate) fn begin(state: &Arc<AtomicU8>) -> Result<BusyClaim, String> {
    BusyClaim::take(state)
}

pub fn read_view(config_path: &Path, state: &Arc<AtomicU8>) -> Result<BrainCliView, String> {
    let config =
        sister_core::config::Config::load(config_path).map_err(|error| format!("{error:#}"))?;
    let configured = config.brain.cli();
    let bridge = configured.and_then(|(_, args)| parse_bridge_args(args));
    // 只有這一版自己寫入、而且 connect 當下真的跑過固定 probe 的 bridge，才叫
    // 「已選用」。舊版手填的 `claude -p` 仍會由 recorder 照原設定使用，但它沒有
    // 通過這條登入流程，不能只看檔名就替它補上一個「已接好」。
    let selected = bridge.as_ref().map(|(provider, _)| *provider);
    let custom_configured = configured.is_some() && selected.is_none();

    let handles: Vec<_> = BrainProvider::ALL
        .into_iter()
        .map(|provider| std::thread::spawn(move || provider_view(provider, selected)))
        .collect();
    let mut providers = Vec::with_capacity(BrainProvider::ALL.len());
    for (provider, handle) in BrainProvider::ALL.into_iter().zip(handles) {
        providers.push(handle.join().unwrap_or(ProviderView {
            id: provider.id(),
            label: provider.label(),
            installed: false,
            version: None,
            selected: selected == Some(provider),
        }));
    }
    Ok(BrainCliView {
        providers,
        selected: selected.map(BrainProvider::id),
        custom_configured,
        busy: state.load(Ordering::Acquire) != JOB_IDLE,
    })
}

fn provider_view(provider: BrainProvider, selected: Option<BrainProvider>) -> ProviderView {
    let executable = detect(provider);
    let version = executable
        .as_ref()
        .and_then(|path| version_of(provider, path));
    ProviderView {
        id: provider.id(),
        label: provider.label(),
        installed: executable.is_some(),
        version,
        selected: selected == Some(provider),
    }
}

pub fn connect(
    provider: BrainProvider,
    config_path: &Path,
    sister_executable: &Path,
    claim: BusyClaim,
) -> Result<BrainCliOutcome, String> {
    let state = Arc::clone(&claim.0);
    let executable =
        detect(provider).ok_or_else(|| format!("找不到 {}；原本的大腦沒有改", provider.label()))?;

    let login = run_managed(
        &executable,
        provider.login_args(),
        None,
        true,
        LOGIN_TIMEOUT,
        &state,
    )?;
    finish_step(provider, "登入", &login)?;
    probe(provider, sister_executable, &executable, &state)?;

    sister_core::config::Config::update(config_path, |config| {
        config.set_brain_cli_from_page(
            sister_executable.to_string_lossy().into_owned(),
            bridge_args(provider, &executable),
        );
        Ok(())
    })
    .map_err(|error| {
        format!(
            "{} 已測通，但沒有存成大腦：{error:#}；原本的大腦沒有改",
            provider.label()
        )
    })?;

    drop(claim);
    Ok(BrainCliOutcome {
        provider: provider.id(),
        label: provider.label(),
        brain: read_view(config_path, &state)?,
    })
}

pub fn test_selected(config_path: &Path, claim: BusyClaim) -> Result<BrainCliOutcome, String> {
    let state = Arc::clone(&claim.0);
    let config =
        sister_core::config::Config::load(config_path).map_err(|error| format!("{error:#}"))?;
    let (command, args) = config.brain.cli().ok_or_else(|| "還沒選大腦".to_owned())?;
    let (provider, provider_executable) = parse_bridge_args(args)
        .ok_or_else(|| "目前的大腦不是這一版接好的 CLI；請重新選一個".to_owned())?;
    probe(provider, Path::new(command), &provider_executable, &state)?;
    drop(claim);
    Ok(BrainCliOutcome {
        provider: provider.id(),
        label: provider.label(),
        brain: read_view(config_path, &state)?,
    })
}

fn probe(
    provider: BrainProvider,
    sister_executable: &Path,
    provider_executable: &Path,
    state: &Arc<AtomicU8>,
) -> Result<(), String> {
    if !sister_executable.is_file() {
        return Err(format!(
            "找不到 sister CLI：{}",
            sister_executable.display()
        ));
    }
    if !provider_executable.is_file() {
        return Err(format!(
            "找不到 {}：{}",
            provider.label(),
            provider_executable.display()
        ));
    }
    let args = bridge_args(provider, provider_executable);
    let output = run_managed(
        sister_executable,
        &args,
        Some(READY_PROMPT.as_bytes()),
        false,
        PROBE_TIMEOUT,
        state,
    )?;
    finish_step(provider, "測試", &output)?;
    if !output_has_ready_token(&output.stdout) {
        return Err(format!(
            "{} 沒有回傳測試碼；原本的大腦沒有改",
            provider.label()
        ));
    }
    Ok(())
}

fn finish_step(provider: BrainProvider, step: &str, output: &RunOutput) -> Result<(), String> {
    if output.cancelled {
        return Err(format!("{}已取消；原本的大腦沒有改", step));
    }
    if output.timed_out {
        return Err(format!(
            "{} {}逾時；原本的大腦沒有改",
            provider.label(),
            step
        ));
    }
    if output.exit_code != Some(0) {
        let result = output.exit_code.map_or_else(
            || "沒有正常結束".to_owned(),
            |code| format!("結束碼 {code}"),
        );
        let detail = first_line(&output.stderr).or_else(|| first_line(&output.stdout));
        return Err(match detail {
            Some(detail) => format!(
                "{} {}沒有完成（{}）：{}；原本的大腦沒有改",
                provider.label(),
                step,
                result,
                detail
            ),
            None => format!(
                "{} {}沒有完成（{}）；原本的大腦沒有改",
                provider.label(),
                step,
                result
            ),
        });
    }
    Ok(())
}

fn first_line(value: &str) -> Option<&str> {
    value.lines().map(str::trim).find(|line| !line.is_empty())
}

pub fn cancel(state: &Arc<AtomicU8>) -> bool {
    loop {
        match state.load(Ordering::Acquire) {
            JOB_IDLE => return false,
            JOB_CANCELLING => return true,
            JOB_ACTIVE => {
                if state
                    .compare_exchange(
                        JOB_ACTIVE,
                        JOB_CANCELLING,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    return true;
                }
            }
            _ => return false,
        }
    }
}

fn version_of(provider: BrainProvider, executable: &Path) -> Option<String> {
    let state = Arc::new(AtomicU8::new(JOB_ACTIVE));
    let output = run_managed(
        executable,
        provider.version_args(),
        None,
        false,
        VERSION_TIMEOUT,
        &state,
    )
    .ok()?;
    if output.exit_code != Some(0) || output.timed_out {
        return None;
    }
    first_line(&output.stdout)
        .or_else(|| first_line(&output.stderr))
        .map(clean_version)
        .filter(|line| !line.is_empty())
}

fn clean_version(line: &str) -> String {
    line.chars()
        .filter(|ch| !ch.is_control())
        .take(96)
        .collect::<String>()
        .trim()
        .to_owned()
}

pub(crate) fn detect(provider: BrainProvider) -> Option<PathBuf> {
    detect_in(
        provider,
        std::env::var_os("PATH").as_deref(),
        common_directories(),
    )
}

fn detect_in(
    provider: BrainProvider,
    path_value: Option<&std::ffi::OsStr>,
    extra_directories: Vec<PathBuf>,
) -> Option<PathBuf> {
    let mut directories = Vec::new();
    if let Some(value) = path_value {
        directories.extend(std::env::split_paths(value));
    }
    directories.extend(extra_directories);
    let mut seen = HashSet::new();
    for directory in directories {
        if !seen.insert(normalized_path_key(&directory)) {
            continue;
        }
        for name in candidate_names(provider) {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return std::fs::canonicalize(&candidate).ok().or(Some(candidate));
            }
        }
    }
    None
}

fn candidate_names(provider: BrainProvider) -> Vec<String> {
    let command = provider.command_name();
    if cfg!(windows) {
        ["exe", "cmd", "bat", ""]
            .into_iter()
            .map(|extension| {
                if extension.is_empty() {
                    command.to_owned()
                } else {
                    format!("{command}.{extension}")
                }
            })
            .collect()
    } else {
        vec![command.to_owned()]
    }
}

fn common_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(path) = std::env::var_os("APPDATA") {
        directories.push(PathBuf::from(path).join("npm"));
    }
    if let Some(path) = std::env::var_os("USERPROFILE") {
        directories.push(PathBuf::from(path).join(".local").join("bin"));
    }
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        directories.push(
            PathBuf::from(path)
                .join("Microsoft")
                .join("WinGet")
                .join("Links"),
        );
    }
    directories
}

fn normalized_path_key(path: &Path) -> String {
    let value = path.to_string_lossy();
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value.into_owned()
    }
}

fn run_managed<S: AsRef<std::ffi::OsStr>>(
    executable: &Path,
    args: &[S],
    input: Option<&[u8]>,
    visible_console: bool,
    timeout: Duration,
    state: &Arc<AtomicU8>,
) -> Result<RunOutput, String> {
    if state.load(Ordering::Acquire) == JOB_CANCELLING {
        return Err("查詢已停止".to_owned());
    }
    let mut command = Command::new(executable);
    command.args(args);
    if visible_console {
        // Windows 的 CREATE_NEW_CONSOLE 會給這個 child 自己的輸入輸出；不要把
        // desktop GUI 的空 handle 明確塞回去。
    } else {
        command
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    }
    sister_core::brain::configure_managed_process(&mut command, visible_console);
    let mut child = command
        .spawn()
        .map_err(|error| format!("啟動 {}：{error}", executable.display()))?;
    let process_tree = sister_core::brain::ManagedProcessTree::attach(&child).map_err(|error| {
        let _ = child.kill();
        let _ = child.wait();
        format!("管理 {} 的子行程：{error}", executable.display())
    })?;
    if let Err(error) = process_tree.resume(&child) {
        process_tree.terminate(&mut child);
        let _ = child.wait();
        return Err(format!("啟動 {}：{error}", executable.display()));
    }

    if let Some(bytes) = input {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "CLI 測試的 stdin 沒開成".to_owned())?;
        if let Err(error) = stdin.write_all(bytes) {
            process_tree.terminate(&mut child);
            let _ = child.wait();
            return Err(format!("寫入 CLI 測試：{error}"));
        }
        drop(stdin);
    }

    let stdout_thread = child.stdout.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            bytes
        })
    });
    let stderr_thread = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            bytes
        })
    });

    let started = Instant::now();
    let mut cancelled = false;
    let mut timed_out = false;
    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => {}
            Err(error) => {
                process_tree.terminate(&mut child);
                let _ = child.wait();
                return Err(format!("等待 {}：{error}", executable.display()));
            }
        }
        if state.load(Ordering::Acquire) == JOB_CANCELLING {
            cancelled = true;
            process_tree.terminate(&mut child);
            break child.wait().ok().and_then(|status| status.code());
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            process_tree.terminate(&mut child);
            break child.wait().ok().and_then(|status| status.code());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    // 正常 direct child 結束時仍可能留 descendants 握著 pipe。關 Job／殺 process
    // group 後再 join，reader 才一定收得回來。
    process_tree.terminate(&mut child);
    let stdout = stdout_thread
        .and_then(|thread| thread.join().ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    let stderr = stderr_thread
        .and_then(|thread| thread.join().ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    Ok(RunOutput {
        stdout,
        stderr,
        exit_code,
        cancelled,
        timed_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn detection_uses_exact_provider_filename_and_path_order() {
        let root = std::env::temp_dir().join(format!(
            "sister-desktop-brain-cli-detect-{}",
            std::process::id()
        ));
        let first = root.join("first");
        let second = root.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let name = candidate_names(BrainProvider::Claude)[0].clone();
        std::fs::write(second.join(&name), b"fixture").unwrap();
        std::fs::write(second.join(format!("not-{name}")), b"fixture").unwrap();
        let path = std::env::join_paths([&first, &second]).unwrap();
        let found = detect_in(BrainProvider::Claude, Some(&path), vec![]).unwrap();
        assert_eq!(found, std::fs::canonicalize(second.join(name)).unwrap());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn extra_directory_is_used_when_path_has_no_provider() {
        let root = std::env::temp_dir().join(format!(
            "sister-desktop-brain-cli-extra-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let name = candidate_names(BrainProvider::Grok)[0].clone();
        std::fs::write(root.join(&name), b"fixture").unwrap();
        let empty = OsString::new();
        assert!(detect_in(BrainProvider::Grok, Some(&empty), vec![root.clone()]).is_some());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn clean_version_removes_controls_and_caps_the_line() {
        let dirty = format!("grok 1.0.13\r{}", "x".repeat(200));
        let clean = clean_version(&dirty);
        assert!(!clean.contains('\r'));
        assert!(clean.chars().count() <= 96);
    }

    #[test]
    fn cancel_without_active_job_does_not_claim_success() {
        let state = Arc::new(AtomicU8::new(JOB_IDLE));
        assert!(!cancel(&state));
        assert_eq!(state.load(Ordering::Acquire), JOB_IDLE);
    }

    #[test]
    fn a_claim_is_visible_before_work_starts_and_cancel_cannot_be_lost() {
        let state = Arc::new(AtomicU8::new(JOB_IDLE));
        let claim = begin(&state).unwrap();
        assert_eq!(state.load(Ordering::Acquire), JOB_ACTIVE);
        assert!(cancel(&state));
        assert_eq!(state.load(Ordering::Acquire), JOB_CANCELLING);
        drop(claim);
        assert_eq!(state.load(Ordering::Acquire), JOB_IDLE);
    }
}

// Login snapshots are memory-only; never included in config, logs or exports.
#[derive(Clone, Debug, Serialize)]
pub struct LoginRow {
    id: &'static str,
    status: String,
}

#[derive(Default)]
pub struct LoginStatus {
    pub state: Arc<AtomicU8>,
    cache: std::sync::Mutex<Option<(Instant, Vec<LoginRow>)>>,
}

impl LoginStatus {
    pub fn cancel(&self) {
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cancel(&self.state);
        *cache = None;
    }

    pub fn read(&self, _claim: BusyClaim, refresh: bool) -> Result<Vec<LoginRow>, String> {
        if !refresh {
            let cache = self.cache.lock().map_err(|_| "問不到；請重新查詢")?;
            if let Some((at, rows)) = cache.as_ref()
                && at.elapsed() < Duration::from_secs(60)
            {
                return Ok(rows.clone());
            }
        }
        let rows = BrainProvider::ALL
            .into_iter()
            .map(|provider| {
                let status = match detect(provider) {
                    None => "未安裝".to_owned(),
                    Some(path) => match provider {
                        BrainProvider::Claude => login_probe(
                            provider,
                            &path,
                            &["auth", "status"],
                            &self.state,
                            VERSION_TIMEOUT,
                        ),
                        BrainProvider::Codex => login_probe(
                            provider,
                            &path,
                            &["login", "status"],
                            &self.state,
                            VERSION_TIMEOUT,
                        ),
                        _ => "已安裝".to_owned(),
                    },
                };
                LoginRow {
                    id: provider.id(),
                    status,
                }
            })
            .collect::<Vec<_>>();
        let mut cache = self.cache.lock().map_err(|_| "問不到；請重新查詢")?;
        if self.state.load(Ordering::Acquire) == JOB_CANCELLING {
            return Err("查詢已停止".to_owned());
        }
        *cache = Some((Instant::now(), rows.clone()));
        Ok(rows)
    }
}

fn login_probe(
    provider: BrainProvider,
    path: &Path,
    args: &[&str],
    state: &Arc<AtomicU8>,
    timeout: Duration,
) -> String {
    run_managed(path, args, None, false, timeout, state)
        .ok()
        .map(|output| login_text(provider, &output))
        .unwrap_or_else(|| "問不到；請重新查詢".to_owned())
}

fn login_text(provider: BrainProvider, output: &RunOutput) -> String {
    let unknown = "問不到；請重新查詢".to_owned();
    if output.timed_out || output.cancelled {
        return unknown;
    }
    match provider {
        BrainProvider::Claude => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Auth {
                logged_in: bool,
                auth_method: Option<String>,
                email: Option<String>,
                subscription_type: Option<String>,
            }
            let Ok(auth) = serde_json::from_str::<Auth>(&output.stdout) else {
                return unknown;
            };
            if !auth.logged_in {
                return "未登入".to_owned();
            }
            if output.exit_code != Some(0) {
                return unknown;
            }
            let clean = |value: Option<String>| {
                value.filter(|s| {
                    !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
                })
            };
            let Some(method) = clean(auth.auth_method) else {
                return unknown;
            };
            format!(
                "已登入（{}） · 帳號：{} · 方案：{}",
                method,
                clean(auth.email).unwrap_or_else(|| "CLI 未提供".to_owned()),
                clean(auth.subscription_type).unwrap_or_else(|| "CLI 未提供".to_owned())
            )
        }
        BrainProvider::Codex => {
            let text = format!("{}\n{}", output.stdout.trim(), output.stderr.trim());
            match (output.exit_code, text.trim()) {
                (Some(0), "Logged in using ChatGPT") => "已登入（ChatGPT）".to_owned(),
                (Some(1), "Not logged in") => "未登入".to_owned(),
                _ => unknown,
            }
        }
        _ => unknown,
    }
}

#[cfg(test)]
mod login_tests {
    use super::*;
    fn output(text: &str) -> RunOutput {
        RunOutput {
            stdout: text.into(),
            stderr: String::new(),
            exit_code: Some(0),
            cancelled: false,
            timed_out: false,
        }
    }
    #[test]
    fn malformed_claude_is_unknown() {
        for text in ["oops", "{}", r#"{"loggedIn":"yes"}"#] {
            assert_eq!(
                login_text(BrainProvider::Claude, &output(text)),
                "問不到；請重新查詢"
            );
        }
    }
    #[test]
    fn login_fields_and_absence_are_honest() {
        assert_eq!(
            login_text(
                BrainProvider::Claude,
                &output(
                    r#"{"loggedIn":true,"authMethod":"oauth","email":"fixture@example.test","subscriptionType":"pro"}"#
                )
            ),
            "已登入（oauth） · 帳號：fixture@example.test · 方案：pro"
        );
        assert_eq!(
            login_text(BrainProvider::Claude, &output(r#"{"loggedIn":false}"#)),
            "未登入"
        );
        assert_eq!(
            login_text(BrainProvider::Codex, &output("Logged in using ChatGPT")),
            "已登入（ChatGPT）"
        );
        assert_eq!(
            login_text(BrainProvider::Codex, &output("new format")),
            "問不到；請重新查詢"
        );
    }
    #[test]
    fn timeout_never_uses_success_text() {
        let mut value = output("Logged in using ChatGPT");
        value.timed_out = true;
        assert_eq!(
            login_text(BrainProvider::Codex, &value),
            "問不到；請重新查詢"
        );
    }
    #[cfg(unix)]
    #[test]
    fn actual_timeout_is_unknown() {
        let state = Arc::new(AtomicU8::new(JOB_ACTIVE));
        let status = login_probe(
            BrainProvider::Codex,
            Path::new("/bin/sh"),
            &["-c", "printf 'Logged in using ChatGPT'; sleep 10"],
            &state,
            Duration::from_millis(50),
        );
        println!(
            "CLI_STATUS_TIMEOUT_ROW={}",
            serde_json::json!({"id": "codex", "status": status})
        );
        assert_eq!(status, "問不到；請重新查詢");
    }
    #[cfg(unix)]
    #[test]
    fn actual_malformed_claude_is_unknown() {
        let state = Arc::new(AtomicU8::new(JOB_ACTIVE));
        assert_eq!(
            login_probe(
                BrainProvider::Claude,
                Path::new("/bin/sh"),
                &["-c", "printf unexpected"],
                &state,
                Duration::from_secs(2)
            ),
            "問不到；請重新查詢"
        );
    }
    #[cfg(unix)]
    #[test]
    fn cancel_terminates_running_probe_and_prevents_next_spawn() {
        let state = Arc::new(AtomicU8::new(JOB_ACTIVE));
        let worker_state = Arc::clone(&state);
        let worker = std::thread::spawn(move || {
            run_managed(
                Path::new("/bin/sh"),
                &["-c", "sleep 30"],
                None,
                false,
                Duration::from_secs(10),
                &worker_state,
            )
        });
        std::thread::sleep(Duration::from_millis(100));
        assert!(cancel(&state));
        let result = worker.join().unwrap().unwrap();
        assert!(result.cancelled);
        assert!(
            run_managed(
                Path::new("/path/that/must/not/be/spawned"),
                &["status"],
                None,
                false,
                Duration::from_secs(1),
                &state
            )
            .unwrap_err()
            .contains("查詢已停止")
        );
    }
    #[test]
    fn cache_cancel_and_restart() {
        let runtime = LoginStatus::default();
        *runtime.cache.lock().unwrap() = Some((
            Instant::now(),
            vec![LoginRow {
                id: "fixture",
                status: "cached".into(),
            }],
        ));
        let rows = runtime.read(begin(&runtime.state).unwrap(), false).unwrap();
        assert_eq!(rows[0].status, "cached");
        runtime.cancel();
        assert!(runtime.cache.lock().unwrap().is_none());
        assert!(LoginStatus::default().cache.lock().unwrap().is_none());
    }
}
