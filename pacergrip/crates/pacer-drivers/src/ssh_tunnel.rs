use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tokio::net::TcpStream;

use pacer_core::{CoreError, CoreResult, SshTunnelConfig};

static ASKPASS_SEQ: AtomicU64 = AtomicU64::new(1);

/// Active SSH local port-forwarding tunnel (`ssh -N -L 127.0.0.1:<local>:<remote_host>:<remote_port>`).
/// Automatically terminates the subprocess and cleans up temporary askpass helper files on drop.
pub struct SshTunnelHandle {
    child: Mutex<Option<Child>>,
    local_port: u16,
    askpass_path: Option<PathBuf>,
}

impl SshTunnelHandle {
    pub fn local_port(&self) -> u16 {
        self.local_port
    }
}

impl Drop for SshTunnelHandle {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut proc) = guard.take() {
                let _ = proc.kill();
                let _ = proc.wait();
            }
        }
        if let Some(ref p) = self.askpass_path {
            let _ = std::fs::remove_file(p);
        }
    }
}

/// Allocates an ephemeral loopback port unless the user specified an explicit positive port.
pub fn allocate_local_port(requested: Option<u16>) -> CoreResult<u16> {
    if let Some(port) = requested.filter(|&p| p > 0) {
        return Ok(port);
    }
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| {
        CoreError::Transport(format!("Failed to bind local port for SSH tunnel: {e}"))
    })?;
    let port = listener
        .local_addr()
        .map_err(|e| CoreError::Transport(format!("Failed reading local tunnel port: {e}")))?
        .port();
    drop(listener);
    Ok(port)
}

/// Expands leading `~/` to the current user's home directory.
pub fn expand_tilde(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("~/").or_else(|| trimmed.strip_prefix("~\\")) {
        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            return Path::new(&home).join(rest).to_string_lossy().into_owned();
        }
    }
    trimmed.to_string()
}

/// Builds clean OpenSSH CLI arguments for local port forwarding.
pub fn build_ssh_args(
    ssh: &SshTunnelConfig,
    local_port: u16,
    remote_host: &str,
    remote_port: u16,
) -> CoreResult<Vec<String>> {
    let host = ssh.host.trim();
    let user = ssh.username.trim();
    if host.is_empty() || user.is_empty() {
        return Err(CoreError::InvalidConfig(
            "SSH 隧道需要填写 SSH 主机和用户名".to_string(),
        ));
    }
    let target_host = remote_host.trim();
    if target_host.is_empty() || remote_port == 0 {
        return Err(CoreError::InvalidConfig(
            "SSH 隧道目标数据库主机或端口无效".to_string(),
        ));
    }

    let ssh_port = if ssh.port == 0 { 22 } else { ssh.port };
    let auth_norm = ssh.auth_type.trim().to_ascii_uppercase();
    let has_secret = ssh
        .password
        .as_deref()
        .is_some_and(|s| !s.is_empty())
        || ssh
            .passphrase
            .as_deref()
            .is_some_and(|s| !s.is_empty());

    let mut args = vec![
        "-N".to_string(),
        "-T".to_string(),
        "-o".to_string(),
        "ExitOnForwardFailure=yes".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(),
        "ServerAliveInterval=30".to_string(),
        "-o".to_string(),
        "ServerAliveCountMax=3".to_string(),
        "-o".to_string(),
        "ConnectTimeout=8".to_string(),
    ];

    if !has_secret && auth_norm != "PASSWORD" {
        args.push("-o".to_string());
        args.push("BatchMode=yes".to_string());
    }

    args.push("-p".to_string());
    args.push(ssh_port.to_string());
    args.push("-L".to_string());
    args.push(format!(
        "127.0.0.1:{local_port}:{target_host}:{remote_port}"
    ));

    if matches!(auth_norm.as_str(), "KEY_PAIR" | "KEY" | "PUBLIC_KEY") {
        if let Some(key_path) = ssh.key_file.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            args.push("-i".to_string());
            args.push(expand_tilde(key_path));
        }
    }

    args.push(format!("{user}@{host}"));
    Ok(args)
}

fn resolve_ssh_executable() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let sys_ssh = PathBuf::from(r"C:\Windows\System32\OpenSSH\ssh.exe");
        if sys_ssh.is_file() {
            return sys_ssh;
        }
    }
    PathBuf::from("ssh")
}

#[cfg(target_os = "windows")]
fn configure_subprocess_platform(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn configure_subprocess_platform(_cmd: &mut Command) {}

fn create_askpass_script(env_var: &str) -> CoreResult<PathBuf> {
    let seq = ASKPASS_SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    #[cfg(target_os = "windows")]
    let path = {
        let p = std::env::temp_dir().join(format!("pacer_askpass_{pid}_{seq}.cmd"));
        let body = format!("@echo off\r\necho %{env_var}%\r\n");
        std::fs::write(&p, body.as_bytes()).map_err(|e| {
            CoreError::Transport(format!("Failed writing temporary SSH askpass helper: {e}"))
        })?;
        p
    };
    #[cfg(not(target_os = "windows"))]
    let path = {
        let p = std::env::temp_dir().join(format!("pacer_askpass_{pid}_{seq}.sh"));
        let body = format!("#!/bin/sh\nprintf '%s\\n' \"${env_var}\"\n");
        std::fs::write(&p, body.as_bytes()).map_err(|e| {
            CoreError::Transport(format!("Failed writing temporary SSH askpass helper: {e}"))
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700));
        }
        p
    };
    Ok(path)
}

/// Opens a live OpenSSH local port-forwarding tunnel and waits until the local forwarded socket accepts connections.
pub async fn open_ssh_tunnel(
    ssh: &SshTunnelConfig,
    remote_host: &str,
    remote_port: u16,
) -> CoreResult<SshTunnelHandle> {
    let local_port = allocate_local_port(ssh.local_port)?;
    let args = build_ssh_args(ssh, local_port, remote_host, remote_port)?;
    let ssh_bin = resolve_ssh_executable();

    let auth_norm = ssh.auth_type.trim().to_ascii_uppercase();
    let secret = if matches!(auth_norm.as_str(), "KEY_PAIR" | "KEY" | "PUBLIC_KEY") {
        ssh.passphrase.as_deref().filter(|s| !s.is_empty())
    } else {
        ssh.password
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| ssh.passphrase.as_deref().filter(|s| !s.is_empty()))
    };

    let mut cmd = Command::new(&ssh_bin);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    configure_subprocess_platform(&mut cmd);

    let mut askpass_path: Option<PathBuf> = None;
    if let Some(secret_val) = secret {
        const ENV_KEY: &str = "PACER_SSH_SECRET";
        let script = create_askpass_script(ENV_KEY)?;
        cmd.env(ENV_KEY, secret_val)
            .env("SSH_ASKPASS", &script)
            .env("SSH_ASKPASS_REQUIRE", "force")
            .env("DISPLAY", ":0");
        askpass_path = Some(script);
    }

    let child = cmd.spawn().map_err(|e| {
        if let Some(ref p) = askpass_path {
            let _ = std::fs::remove_file(p);
        }
        CoreError::Transport(format!(
            "启动 SSH 隧道进程失败 ({}): {e}。请确认系统已安装 OpenSSH 客户端 (`ssh`)。",
            ssh_bin.display()
        ))
    })?;

    let handle = SshTunnelHandle {
        child: Mutex::new(Some(child)),
        local_port,
        askpass_path,
    };

    let deadline = Instant::now() + Duration::from_secs(9);
    loop {
        if TcpStream::connect(("127.0.0.1", local_port)).await.is_ok() {
            return Ok(handle);
        }

        if let Ok(mut guard) = handle.child.lock() {
            if let Some(ref mut proc) = *guard {
                if let Ok(Some(status)) = proc.try_wait() {
                    let mut err_buf = String::new();
                    if let Some(mut stderr) = proc.stderr.take() {
                        use std::io::Read;
                        let _ = stderr.read_to_string(&mut err_buf);
                    }
                    let detail = err_buf.trim();
                    return Err(CoreError::Transport(format!(
                        "SSH 隧道连接失败 (exit {status}): {}",
                        if detail.is_empty() {
                            "请检查跳板机地址、端口、私钥权限或口令"
                        } else {
                            detail
                        }
                    )));
                }
            }
        }

        if Instant::now() >= deadline {
            return Err(CoreError::Transport(format!(
                "SSH 隧道建立超时 ({}@{} -> {remote_host}:{remote_port})",
                ssh.username, ssh.host
            )));
        }

        tokio::time::sleep(Duration::from_millis(80)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ssh_args_key_and_password_modes() {
        let key_cfg = SshTunnelConfig {
            id: None,
            config_name: None,
            host: "bastion.prod.internal".to_string(),
            port: 2222,
            username: "deploy".to_string(),
            auth_type: "KEY_PAIR".to_string(),
            key_file: Some("/tmp/id_ed25519".to_string()),
            local_port: None,
            enabled: true,
            password: None,
            passphrase: None,
        };
        let args = build_ssh_args(&key_cfg, 45123, "10.0.1.5", 3306).unwrap();
        assert!(args.contains(&"-L".to_string()));
        assert!(args.contains(&"127.0.0.1:45123:10.0.1.5:3306".to_string()));
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"/tmp/id_ed25519".to_string()));
        assert!(args.contains(&"BatchMode=yes".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("deploy@bastion.prod.internal"));

        let pwd_cfg = SshTunnelConfig {
            auth_type: "PASSWORD".to_string(),
            key_file: None,
            password: Some("s3cr3t".to_string()),
            ..key_cfg
        };
        let pwd_args = build_ssh_args(&pwd_cfg, 45124, "db.internal", 5432).unwrap();
        assert!(!pwd_args.contains(&"BatchMode=yes".to_string()));
        assert!(pwd_args.contains(&"127.0.0.1:45124:db.internal:5432".to_string()));
    }

    #[test]
    fn test_build_ssh_args_rejects_missing_host_or_port() {
        let bad = SshTunnelConfig {
            id: None,
            config_name: None,
            host: "".to_string(),
            port: 22,
            username: "ops".to_string(),
            auth_type: "AGENT".to_string(),
            key_file: None,
            local_port: None,
            enabled: true,
            password: None,
            passphrase: None,
        };
        assert!(build_ssh_args(&bad, 15000, "127.0.0.1", 3306).is_err());

        let free_port = allocate_local_port(None).unwrap();
        assert!(free_port > 0);
    }
}
