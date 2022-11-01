mod routes;

use crate::rocket_utils::CmdRequest;
use crate::{dock::*, grpc, images, logs};
use anyhow::Result;
use bollard::Docker;
use rocket::tokio::sync::{mpsc, Mutex};
use std::sync::Arc;

pub async fn run(docker: Docker) -> Result<()> {
    let proj = "stack";
    let network = "regtest";

    // btc setup
    let btc_node = images::BtcNode::new("bitcoind", network, "foo", "bar");
    let btc1 = images::btc(proj, &btc_node);
    let btc_id = create_and_start(&docker, btc1).await?;
    log::info!("created bitcoind");

    // lnd setup
    let http_port = "38881";
    let lnd_node = images::LndNode::new("lnd1", network, "10009", "/root/.lnd");
    let lnd1 = images::lnd(proj, &lnd_node, &btc_node, Some(http_port));
    let lnd_id = create_and_start(&docker, lnd1).await?;
    log::info!("created LND");

    let unlocker = grpc::lnd::LndUnlocker::new(http_port).await?;
    let res = unlocker.init_wallet().await?;

    let (tx, _rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();

    // launch rocket
    let port = std::env::var("ROCKET_PORT").unwrap_or("8000".to_string());
    log::info!("🚀 => http://localhost:{}", port);
    let log_txs = Arc::new(Mutex::new(log_txs));
    let _r = routes::launch_rocket(tx.clone(), log_txs).await;

    // shutdown containers
    remove_container(&docker, &btc_id).await?;
    remove_container(&docker, &lnd_id).await?;

    Ok(())
}
