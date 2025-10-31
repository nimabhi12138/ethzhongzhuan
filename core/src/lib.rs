use clap::crate_version;
use tokio::time::Instant;
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;
const SPLIT: u8 = b'\n';

// 已移除开发者钱包地址：0x98be5c44d574b96b320dffb0ccff116bda433b8e
lazy_static! {
    pub static ref JWT_SECRET: String = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| {
            "default_jwt_secret_please_change_in_env".into()
        });
}

// 已禁用：开发者工作矿机名称（原用于开发者抽水）
lazy_static! {
    pub static ref DEVELOP_WORKER_NAME: String = {
        let name = match hostname::get() {
            Ok(name) => {
                "worker_".to_string()
                    + name.to_str().expect("无法将机器名称转为字符串")
            }
            Err(_) => crate_version!().to_string().replace(".", ""),
        };
        name
    };
}

// 已禁用：开发者费用（原默认值为 0.02 即 2%）
lazy_static! {
    pub static ref DEVELOP_FEE: f64 = match std::env::var("DEVELOP_FEE") {
        Ok(fee) => {
            fee.parse().unwrap()
        }
        Err(_) => 0.0,  // 改为 0，不再收取开发者费用
    };
}

lazy_static! {
    pub static ref RUNTIME: tokio::time::Instant = Instant::now();
}

pub fn init() {
    let a = RUNTIME.elapsed().as_secs();
    a.to_string();
    let name = &DEVELOP_WORKER_NAME;
    name.to_string();
    let jwt_secret = &JWT_SECRET;
    jwt_secret.to_string();
    let dev_fee = &DEVELOP_FEE;
    dev_fee.to_string();
}

pub mod client;
pub mod protocol;
pub mod proxy;
pub mod state;
pub mod util;
pub mod web;
