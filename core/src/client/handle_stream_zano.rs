use std::{collections::HashMap, sync::Arc};

use anyhow::{bail, Result};
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, WriteHalf},
    select,
    time::{self, Duration},
};
use tracing::warn;

use crate::{
    protocol::PROTOCOL,
    proxy::Proxy,
    state::Worker,
};

use super::{lines_unwrap, write_string};

pub async fn handle_stream_zano<R, W, PR, PW>(
    worker: &mut Worker,
    worker_r: tokio::io::BufReader<tokio::io::ReadHalf<R>>,
    mut worker_w: WriteHalf<W>,
    pool_r: tokio::io::BufReader<tokio::io::ReadHalf<PR>>,
    mut pool_w: WriteHalf<PW>,
    proxy: Arc<Proxy>,
    is_encrypted: bool,
) -> Result<()>
where
    R: AsyncRead,
    W: AsyncWrite,
    PR: AsyncRead,
    PW: AsyncWrite,
{
    let fallback_worker_name = {
        let cfg = proxy.config.read().await;
        cfg.share_name.clone()
    };

    let mut worker_lines = worker_r.lines();
    let mut pool_lines = pool_r.lines();

    let mut worker_display_name: String = String::new();
    let mut pending_methods: HashMap<String, String> = HashMap::new();

    let workers_queue = proxy.worker_tx.clone();
    let mut heartbeat = time::sleep(Duration::from_secs(30));
    tokio::pin!(heartbeat);

    loop {
        select! {
            res = worker_lines.next_line() => {
                let buffer = lines_unwrap(res, &worker_display_name, "矿机").await?;
                if buffer.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<Value>(&buffer) {
                    Ok(json) => {
                        if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                            if let Some(id) = json.get("id") {
                                pending_methods.insert(id_to_key(id), method.to_string());
                            }

                            match method {
                                "login" => {
                                    if let Some(params) = json.get("params") {
                                        if let Some((full_worker, short_worker, wallet)) =
                                            parse_login_fields(params, &fallback_worker_name)
                                        {
                                            worker.set_protocol(PROTOCOL::ZANO);
                                            worker.login(full_worker.clone(), short_worker, wallet);
                                            worker_display_name = full_worker;
                                        } else {
                                            warn!("无法解析矿机登录参数: {}", buffer);
                                            bail!("矿机登录参数缺失");
                                        }
                                    }
                                }
                                "submit" => {
                                    worker.share_index_add();
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(err) => {
                        warn!("ZANO 矿机请求解析失败: {} -- {}", err, buffer);
                    }
                }

                write_string(is_encrypted, &mut pool_w, &buffer, &worker_display_name).await?;
            },
            res = pool_lines.next_line() => {
                let buffer = lines_unwrap(res, &worker_display_name, "矿池").await?;
                if buffer.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<Value>(&buffer) {
                    Ok(json) => {
                        if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                            if method == "job" {
                                if let Err(e) = worker.send_job() {
                                    warn!("记录作业失败: {}", e);
                                }
                            }

                            write_string(is_encrypted, &mut worker_w, &buffer, &worker_display_name).await?;
                        } else if let Some(id) = json.get("id") {
                            let key = id_to_key(id);
                            if let Some(original_method) = pending_methods.remove(&key) {
                                match original_method.as_str() {
                                    "login" => {
                                        if is_result_ok(json.get("result")) {
                                            worker.logind();
                                        }
                                    }
                                    "submit" => {
                                        if is_result_ok(json.get("result")) {
                                            worker.share_accept();
                                        } else {
                                            worker.share_reject();
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            write_string(is_encrypted, &mut worker_w, &buffer, &worker_display_name).await?;
                        } else {
                            write_string(is_encrypted, &mut worker_w, &buffer, &worker_display_name).await?;
                        }
                    }
                    Err(err) => {
                        warn!("ZANO 矿池响应解析失败: {} -- {}", err, buffer);
                        write_string(is_encrypted, &mut worker_w, &buffer, &worker_display_name).await?;
                    }
                }
            },
            () = &mut heartbeat => {
                if let Err(e) = workers_queue.send(worker.clone()) {
                    warn!("发送矿工状态失败: {}", e);
                }
                heartbeat.as_mut().reset(time::Instant::now() + Duration::from_secs(30));
            }
        }
    }
}

fn id_to_key(value: &Value) -> String {
    if let Some(s) = value.as_str() {
        s.to_string()
    } else {
        value.to_string()
    }
}

fn parse_login_fields(params: &Value, fallback_worker: &str) -> Option<(String, String, String)> {
    let login_value = match params.get("login").and_then(|v| v.as_str()) {
        Some(v) => v,
        None => return None,
    };

    let (wallet, worker_label_opt) = split_wallet_and_worker(login_value);
    let worker_label = worker_label_opt
        .or_else(|| params.get("rigid").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| fallback_worker.to_string());

    let full_worker = if worker_label.is_empty() {
        wallet.clone()
    } else {
        format!("{}.{}", wallet, worker_label)
    };

    Some((full_worker, worker_label, wallet))
}

fn split_wallet_and_worker(login: &str) -> (String, Option<String>) {
    match login.split_once('.') {
        Some((wallet, worker)) => (wallet.to_string(), Some(worker.to_string())),
        None => (login.to_string(), None),
    }
}

fn is_result_ok(result: Option<&Value>) -> bool {
    match result {
        Some(Value::Bool(true)) => true,
        Some(Value::String(s)) => s.eq_ignore_ascii_case("OK"),
        Some(Value::Object(map)) => map.get("status")
            .and_then(|status| status.as_str())
            .map(|s| s.eq_ignore_ascii_case("OK"))
            .unwrap_or(false),
        _ => false,
    }
}
