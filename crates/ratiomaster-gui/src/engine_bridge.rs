use std::path::PathBuf;
use std::time::Instant;

use ratiomaster_core::client::profiles;
use ratiomaster_core::engine::speed::SpeedConfig;
use ratiomaster_core::engine::stop::StopCondition;
use ratiomaster_core::engine::{Engine, EngineConfig};
use ratiomaster_core::proxy::ProxyConfig;
use ratiomaster_core::torrent;

use crate::tabs::{
    parse_u16, parse_u64, EngineHandles, TabStatus, TorrentTab, PROXY_TYPES, STOP_TYPES,
};

/// Start the engine for a given tab. Returns handles on success, or an error message.
pub fn start_engine(tab: &mut TorrentTab, runtime: &tokio::runtime::Runtime) -> Result<(), String> {
    if tab.status == TabStatus::Running {
        return Err("Engine already running".into());
    }

    let torrent_data = tab
        .torrent_data
        .as_ref()
        .ok_or("No torrent loaded")?
        .clone();

    let torrent_meta =
        torrent::parse(&torrent_data).map_err(|e| format!("Failed to parse torrent: {e}"))?;

    let client_name = tab
        .client_names
        .get(tab.selected_client)
        .cloned()
        .unwrap_or_default();
    let profile = profiles::get_profile(&client_name)
        .ok_or(format!("Unknown client: {client_name}"))?
        .clone();

    let stop_condition = build_stop_condition(tab);
    let speed = build_speed_config(tab);
    let proxy = build_proxy_config(tab);

    let engine_config = EngineConfig {
        port: parse_u16(&tab.port),
        speed,
        stop_condition,
        ignore_failure: tab.ignore_failure,
        max_retries: 5,
        initial_downloaded_percent: 0,
        http_timeout: std::time::Duration::from_secs(30),
        bind_address: "127.0.0.1".into(),
    };

    let mut engine = Engine::new(torrent_meta, profile, proxy, engine_config);
    let state_rx = engine.subscribe_state();
    let shutdown_tx = engine.shutdown_handle();
    let force_announce_tx = engine.force_announce_handle();

    tab.handles = Some(EngineHandles {
        shutdown_tx,
        force_announce_tx,
        state_rx,
    });
    tab.status = TabStatus::Running;
    tab.started_at = Some(Instant::now());

    runtime.spawn(async move {
        if let Err(e) = engine.run().await {
            tracing::error!("engine error: {e}");
        }
    });

    Ok(())
}

/// Stop the engine for a given tab.
pub fn stop_engine(tab: &mut TorrentTab) {
    if let Some(ref handles) = tab.handles {
        let _ = handles.shutdown_tx.send(true);
    }
    tab.status = TabStatus::Stopped;
}

/// Force an announce for a given tab.
pub fn force_announce(tab: &TorrentTab, runtime: &tokio::runtime::Runtime) {
    if let Some(ref handles) = tab.handles {
        let tx = handles.force_announce_tx.clone();
        runtime.spawn(async move {
            let _ = tx.send(()).await;
        });
    }
}

/// Load a torrent file into a tab.
pub fn load_torrent(tab: &mut TorrentTab, path: PathBuf) -> Result<(), String> {
    let data = std::fs::read(&path).map_err(|e| format!("Failed to read file: {e}"))?;
    let meta = torrent::parse(&data).map_err(|e| format!("Failed to parse torrent: {e}"))?;

    let hash_str = meta
        .info_hash
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    let total_size = meta.total_size();
    tab.name = meta.name.clone();
    tab.torrent_path = Some(path);
    tab.torrent_data = Some(data);
    tab.torrent_name = meta.name;
    tab.tracker_url = meta.announce;
    tab.info_hash = hash_str;
    tab.total_size = total_size;

    Ok(())
}

fn build_stop_condition(tab: &TorrentTab) -> StopCondition {
    let label = STOP_TYPES.get(tab.stop_type).copied().unwrap_or("Never");
    match label {
        "After Upload" => StopCondition::AfterUpload(parse_u64(&tab.stop_value) * 1024 * 1024),
        "After Download" => StopCondition::AfterDownload(parse_u64(&tab.stop_value) * 1024 * 1024),
        "After Time" => {
            StopCondition::AfterTime(std::time::Duration::from_secs(parse_u64(&tab.stop_value)))
        }
        "After Seeders" => StopCondition::AfterSeeders(tab.stop_value.parse().unwrap_or(0)),
        "After Leechers" => StopCondition::AfterLeechers(tab.stop_value.parse().unwrap_or(0)),
        "After Ratio" => StopCondition::AfterRatio(tab.stop_value.parse().unwrap_or(0.0)),
        _ => StopCondition::Never,
    }
}

fn build_speed_config(tab: &TorrentTab) -> SpeedConfig {
    let upload_speed = parse_u64(&tab.upload_speed);
    let download_speed = parse_u64(&tab.download_speed);

    let upload_min = if tab.upload_random {
        parse_u64(&tab.upload_random_min)
    } else {
        upload_speed
    };
    let upload_max = if tab.upload_random {
        parse_u64(&tab.upload_random_max)
    } else {
        upload_speed
    };
    let download_min = if tab.download_random {
        parse_u64(&tab.download_random_min)
    } else {
        download_speed
    };
    let download_max = if tab.download_random {
        parse_u64(&tab.download_random_max)
    } else {
        download_speed
    };

    SpeedConfig {
        upload_min: upload_min * 1024,
        upload_max: upload_max * 1024,
        download_min: download_min * 1024,
        download_max: download_max * 1024,
        variation: 10 * 1024,
    }
}

fn build_proxy_config(tab: &TorrentTab) -> ProxyConfig {
    let label = PROXY_TYPES.get(tab.proxy_type).copied().unwrap_or("None");
    match label {
        "SOCKS4" => ProxyConfig::Socks4 {
            proxy_host: tab.proxy_host.clone(),
            proxy_port: parse_u16(&tab.proxy_port),
            user_id: tab.proxy_user.clone(),
        },
        "SOCKS4a" => ProxyConfig::Socks4a {
            proxy_host: tab.proxy_host.clone(),
            proxy_port: parse_u16(&tab.proxy_port),
            user_id: tab.proxy_user.clone(),
        },
        "SOCKS5" => {
            let credentials = if tab.proxy_user.is_empty() {
                None
            } else {
                Some(ratiomaster_core::proxy::socks5::Credentials {
                    username: tab.proxy_user.clone(),
                    password: tab.proxy_pass.clone(),
                })
            };
            ProxyConfig::Socks5 {
                proxy_host: tab.proxy_host.clone(),
                proxy_port: parse_u16(&tab.proxy_port),
                credentials,
            }
        }
        "HTTP Connect" => {
            let credentials = if tab.proxy_user.is_empty() {
                None
            } else {
                Some(ratiomaster_core::proxy::http::Credentials {
                    username: tab.proxy_user.clone(),
                    password: tab.proxy_pass.clone(),
                })
            };
            ProxyConfig::HttpConnect {
                proxy_host: tab.proxy_host.clone(),
                proxy_port: parse_u16(&tab.proxy_port),
                credentials,
            }
        }
        _ => ProxyConfig::None,
    }
}
