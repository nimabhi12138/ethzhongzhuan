mod version {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
}

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

use rustls_pemfile::{certs, rsa_private_keys};
use tokio_rustls::rustls::{self, Certificate, PrivateKey};

use std::{path::Path, sync::Arc, collections::VecDeque};
use tracing::Level;

use tokio::sync::{broadcast, RwLock, Mutex};

use tracing_subscriber::{
    self,
    fmt::{format::Writer, time::FormatTime},
};

use dotenv::dotenv;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::OpenOptions, io::Read};



use actix_web::{dev::ServiceRequest, web, App, Error, HttpServer};

use core::{
    client::{
        encry::accept_en_tcp, tcp::accept_tcp, tls::accept_tcp_with_tls, SSL,
        TCP,
    },
    proxy::Job,
    state::Worker,
    util::config::Settings,
    web::{handles::auth::Claims, AppState, OnlineWorker},
};

use anyhow::{bail, Result};

use clap::{crate_version, ArgMatches};
//use crossbeam_channel::bounded;
use human_panic::setup_panic;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    select,
    sync::mpsc::{self, UnboundedReceiver},
};

fn main() -> Result<()> {
    setup_panic!();
    dotenv().ok();

    if std::fs::metadata("./logs/").is_err() {
        std::fs::create_dir("./logs/")?;
    }

    struct LocalTimer;
    impl FormatTime for LocalTimer {
        fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
            write!(w, "{}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))
        }
    }

    let file_appender =
        tracing_appender::rolling::daily("./logs/", "mining_proxy");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 设置日志输出时的格式，例如，是否包含日志级别、是否包含日志来源位置、
    // 设置日志的时间格式 参考: https://docs.rs/tracing-subscriber/0.3.3/tracing_subscriber/fmt/struct.SubscriberBuilder.html#method.with_timer
    let format = tracing_subscriber::fmt::format()
        .with_level(true)
        .with_target(false)
        .with_line_number(true)
        .with_source_location(true)
        .with_timer(LocalTimer);

    // 初始化并设置日志格式(定制和筛选日志)
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        //.with_writer(io::stdout) // 写入标准输出
        .with_writer(non_blocking) // 写入文件，将覆盖上面的标准输出
        .with_ansi(false) // 如果日志是写入文件，应将ansi的颜色输出功能关掉
        .event_format(format)
        .init();

    core::init();
    let matches = core::util::get_app_command_matches()?;
    if !matches.is_present("server") {
        tracing::info!(
            "版本: {} commit: {} {}",
            crate_version!(),
            version::commit_date(),
            version::short_sha(),
        );

        actix_web::rt::System::with_tokio_rt(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                //.worker_threads(1)
                .thread_name("main-tokio")
                .build()
                .unwrap()
        })
        .block_on(async_main(matches))?;
    } else {
        //tokio::runtime::start
        tokio_main(&matches)?;
    }
    Ok(())
}

async fn async_main(_matches: ArgMatches<'_>) -> Result<()> {
    let data: AppState = Arc::new(std::sync::Mutex::new(HashMap::new()));

    match OpenOptions::new()
        .write(true)
        .read(true)
        .open("configs.yaml")
    {
        Ok(mut f) => {
            let mut configs = String::new();
            if let Ok(len) = f.read_to_string(&mut configs) {
                if len > 0 {
                    let configs: Vec<Settings> =
                        match serde_yaml::from_str(&configs) {
                            Ok(s) => s,
                            Err(e) => {
                                tracing::error!("{}", e);
                                vec![]
                            }
                        };
                    for config in configs {
                        match core::util::run_server(&config) {
                            Ok(child) => {
                                let online = OnlineWorker {
                                    child,
                                    config: config.clone(),
                                    workers: vec![],
                                    online: 0,
                                };

                                data.lock()
                                    .unwrap()
                                    .insert(config.name, online);
                            }
                            Err(e) => {
                                tracing::error!("{}", e);
                            }
                        }
                    }
                }
            }
        }
        Err(_) => {}
    };

    let tcp_data = data.clone();

    tokio::spawn(async move { recv_from_child(tcp_data).await });

    let port: i32 = match std::env::var("MINING_PROXY_WEB_PORT") {
        Ok(p) => p.parse().unwrap(),
        Err(_) => 8888,
    };

    let http_data = data.clone();
    let web_sever = if let Ok(http) = HttpServer::new(move || {
        let generated = generate();

        use actix_web_grants::GrantsMiddleware;
        let auth = GrantsMiddleware::with_extractor(extract);

        App::new()
            .wrap(auth)
            .app_data(web::Data::new(http_data.clone()))
            .service(
                web::scope("/api")
                    .service(core::web::handles::user::login)
                    .service(core::web::handles::user::info)
                    .service(core::web::handles::user::logout)
                    .service(core::web::handles::server::crate_app)
                    .service(core::web::handles::server::server_list)
                    .service(core::web::handles::server::server)
                    .service(core::web::handles::server::dashboard),
            )
            .service(actix_web_static_files::ResourceFiles::new("/", generated))
    })
    .bind(format!("0.0.0.0:{}", port))
    {
        http.run()
    } else {
        let mut proxy_server = data.lock().unwrap();
        for (_, other_server) in &mut *proxy_server {
            other_server.child.kill().await?;
        }
        bail!("web端口 {} 被占用了", port);
    };

    tracing::info!("界面启动成功地址为: {}", format!("0.0.0.0:{}", port));
    web_sever.await?;
    Ok(())
}

fn tokio_main(matches: &ArgMatches<'_>) -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { tokio_run(matches).await })?;
    Ok(())
}

async fn tokio_run(matches: &ArgMatches<'_>) -> Result<()> {
    let config_file_name = matches.value_of("config").unwrap_or("default.yaml");
    let config = Settings::new(config_file_name, true)?;

    match config.check().await {
        Ok(_) => {}
        Err(err) => {
            tracing::error!("config配置错误 {}", err);
            std::process::exit(1);
        }
    };

    match config.check_net_work().await {
        Ok(_) => {}
        Err(err) => {
            tracing::error!("网络错误 {}", err);
        }
    };

    // let cert = match std::fs::File::open(config.pem_path.clone()).await {
    //     Ok(f) => {
    //         tracing::info!("读取到自定义证书: {}", config.pem_path);
    //         // let cert = certs(&mut std::io::BufReader::new(f))
    //         // .map_err(|_| {
    //         //     // std::io::Error::new(
    //         //     //     std::io::ErrorKind::InvalidInput,
    //         //     //     "{} pem证书失败,请使用正确的pem证书!!!",
    //         //     // )
    //         //     bail!("{} pem证书失败,请使用正确的pem证书!!!")
    //         // })
    //         // .map(|mut certs| certs.drain(..).map(Certificate).collect();
    //         load_certs()

    //         Some(f)
    //     }
    //     Err(_) => {
    //         tracing::info!("未读取到证书: {},使用默认证书", config.pem_path);
    //         None
    //     }
    // };

    // let key = match File::open(config.key_path.clone()).await {
    //     Ok(f) => {
    //         tracing::info!("读取到自定义证书: {}", config.key_path);

    //         Some(f)
    //     }
    //     Err(_) => {
    //         tracing::info!("未读取到证书: {},使用默认证书", config.key_path);
    //         None
    //     }
    // };

    let mode = if config.share == 0 {
        "纯代理模式"
    } else if config.share == 1 {
        "抽水模式"
    } else {
        "统一钱包模式"
    };

    tracing::info!("名称 {} 当前启动模式为: {}", config.name, mode);

    let worker_name = config.share_name.clone();

    let (stream_type, _) = match core::client::get_pool_ip_and_type_from_vec(
        &config.share_address,
    ) {
        Ok((stream, addr)) => (stream, addr),
        Err(e) => {
            tracing::error!("Share_address 矿池参数格式化失败。无法启动 {}", e);
            return Ok(());
        }
    };

    let certs = match load_certs(Path::new(&config.pem_path)) {
        Ok(cert) => {
            // tracing::info!(
            //     "自定义SSL证书 {} 读取成功。使用此证书.",
            //     config.pem_path
            // );
            cert
        }
        Err(_) => {
            // tracing::error!(
            //     "自定义SSL证书 {} 未找到或格式不正确.将使用默认证书",
            //     config.pem_path
            // );
            //panic!("未找到默认证书pem");

            tracing::info!(
                "自定义SSL证书 {} 读取失败。请设置证书。未设置程序将退出。。",
                config.pem_path
            );
            std::process::exit(1);
        }
    };

    let mut keys = match load_keys(Path::new(&config.key_path)) {
        Ok(key) => {
            // tracing::info!(
            //     "自定义秘钥key {} 读取成功。使用此秘钥。",
            //     config.key_path
            // );
            key
        }
        Err(_) => {
            // tracing::error!(
            //     "自定义秘钥key {} 未找到或格式不正确.将使用默认证书",
            //     config.key_path
            // );
            tracing::info!(
                "自定义秘钥key {} 读取失败。请设置证书。未设置程序将退出。。",
                config.key_path
            );
            std::process::exit(1);
            //panic!("未找到默认Key");
            //let key_pem = include_bytes!("key.pem");
        }
    };

    let cert_config = match rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, keys.remove(0))
        .map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, err)
        }) {
        Ok(conf) => {
            // tracing::info!(
            //     "自定义SSL证书 {} 格式正确。使用此证书作为SSL证书",
            //     config.pem_path
            // );
            conf
        }
        Err(e) => {
            tracing::info!("证书格式化失败。 请修改证书: {}", e);
            std::process::exit(1);
        }
    };

    //    if config.coin == "ETH" {
    // let (chan_tx, _chan_rx) = broadcast::channel::<Vec<String>>(1);
    // let (dev_chan_tx, _dev_chan_rx) = broadcast::channel::<Vec<String>>(1);
    let fee_job:Job = Arc::new(RwLock::new(VecDeque::new()));
    let develop_job:Job = Arc::new(RwLock::new(VecDeque::new()));
    
    let (tx, rx) = mpsc::channel::<Vec<String>>(15);
    let (dev_tx, dev_rx) = mpsc::channel::<Vec<String>>(15);
    // let (tx, rx) =
    //     bounded::<Vec<String>>(15);
    // let (dev_tx, dev_rx) =
    //     bounded::<Vec<String>>(15);
    tracing::debug!("创建矿工队列");
    // 旷工状态发送队列
    let (worker_tx, worker_rx) = mpsc::unbounded_channel::<Worker>();

    let mconfig = config.clone();
    let proxy = Arc::new(core::proxy::Proxy {
        config: Arc::new(RwLock::new(config)),
        worker_tx,
//        chan: chan_tx.clone(),
        tx,
        dev_tx,
	fee_job:fee_job.clone(),
	develop_job:develop_job.clone(),
//        dev_chan: dev_chan_tx.clone(),
    });

    // 已删除：开发者抽水登录和线程
    // let (dev_lines, dev_w) =
    //     core::client::dev_pool_ssl_login(core::DEVELOP_WORKER_NAME.to_string())
    //         .await?;

    if stream_type == TCP {
        let (proxy_lines, proxy_w) =
            core::client::proxy_pool_login(&mconfig, worker_name.clone())
                .await?;
        let res = tokio::try_join!(
            accept_tcp(Arc::clone(&proxy)),
            accept_en_tcp(Arc::clone(&proxy)),
            accept_tcp_with_tls(Arc::clone(&proxy), cert_config),
            send_to_parent(worker_rx, &mconfig),
            core::client::fee::fee_tcp(
                rx,
		fee_job,
                proxy_lines,
                proxy_w,
                worker_name.clone(),
		proxy.clone(),
            ),
            // 已删除：开发者抽水线程 develop_fee_ssl
        );

        if let Err(err) = res {
            tracing::error!("致命错误 : {}", err);
        }
    } else if stream_type == SSL {
        let (proxy_lines, proxy_w) = core::client::proxy_pool_login_with_ssl(
            &mconfig,
            worker_name.clone(),
        )
        .await?;

        let res = tokio::try_join!(
            accept_tcp(Arc::clone(&proxy)),
            accept_en_tcp(Arc::clone(&proxy)),
            accept_tcp_with_tls(Arc::clone(&proxy), cert_config),
            send_to_parent(worker_rx, &mconfig),
            core::client::fee::fee_ssl(
                rx,
		fee_job,
                proxy_lines,
                proxy_w,
                worker_name.clone(),
		proxy.clone(),
            ),
            // 已删除：开发者抽水线程 develop_fee_ssl
        );

        if let Err(err) = res {
            tracing::error!("致命错误 : {}", err);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendToParentStruct {
    name: String,
    worker: Worker,
}

async fn send_to_parent(
    mut worker_rx: UnboundedReceiver<Worker>, config: &Settings,
) -> Result<()> {
    loop {
        if let Ok(mut stream) =
            tokio::net::TcpStream::connect("127.0.0.1:65501").await
        {
            //let name = config.name.clone();
            loop {
                select! {
                    Some(w) = worker_rx.recv() => {
                        let send = SendToParentStruct{
                            name:config.name.clone(),
                            worker:w,
                        };
                        let mut rpc = serde_json::to_vec(&send)?;
                        rpc.push(b'\n');
                        stream.write(&rpc).await.unwrap();
                    },
                }
            }
        } else {
            tracing::error!("无法链接到主控web端");
            tokio::time::sleep(tokio::time::Duration::from_secs(60 * 2)).await;
        }
    }
}

async fn recv_from_child(app: AppState) -> Result<()> {
    let address = "127.0.0.1:65501";
    let listener = match tokio::net::TcpListener::bind(address.clone()).await {
        Ok(listener) => listener,
        Err(_) => {
            tracing::info!("本地端口被占用 {}", address);
            std::process::exit(1);
        }
    };

    tracing::info!("本地TCP端口{} 启动成功!!!", &address);
    loop {
        let (mut stream, _) = listener.accept().await?;
        let inner_app = app.clone();

        tokio::spawn(async move {
            let (r, _) = stream.split();
            let r_buf = BufReader::new(r);
            let mut r_lines = r_buf.lines();

            loop {
                if let Ok(Some(buf_str)) = r_lines.next_line().await {
                    if let Ok(online_work) =
                        serde_json::from_str::<SendToParentStruct>(&buf_str)
                    {
                        #[cfg(debug_assertions)]
                        dbg!("{}", &online_work);

                        if let Some(temp_app) =
                            inner_app.lock().unwrap().get_mut(&online_work.name)
                        {
                            let mut is_update = false;
                            for worker in &mut temp_app.workers {
                                if worker.worker == online_work.worker.worker {
                                    //dbg!(&worker);
                                    *worker = online_work.worker.clone();
                                    is_update = true;
                                }
                            }
                            if !is_update {
                                temp_app.workers.push(online_work.worker);
                            }
                        } else {
                            tracing::error!("未找到此端口");
                        }
                    }
                };
            }
        });
    }
}

use core::JWT_SECRET;

const ROLE_ADMIN: &str = "ROLE_ADMIN";
// You can use both &ServiceRequest and &mut ServiceRequest
async fn extract(req: &mut ServiceRequest) -> Result<Vec<String>, Error> {
    // Here is a place for your code to get user permissions/grants/permissions
    // from a request For example from a token or database
    // tracing::info!("check the Role");
    // println!("{:?}", req.headers().get("token"));

    if req.path() != "/api/user/login" {
        // 判断权限
        if let Some(token) = req.headers().get("token") {
            let token_data = decode::<Claims>(
                token.to_str().unwrap(),
                &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
                &Validation::default(),
            );
            if let Ok(_) = token_data {
                Ok(vec![ROLE_ADMIN.to_string()])
            } else {
                Ok(vec![])
            }
        } else {
            Ok(vec![])
        }
    } else {
        Ok(vec![])
    }
}

fn load_certs(path: &Path) -> std::io::Result<Vec<Certificate>> {
    certs(&mut std::io::BufReader::new(std::fs::File::open(path)?))
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid cert",
            )
        })
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

fn load_keys(path: &Path) -> std::io::Result<Vec<PrivateKey>> {
    rsa_private_keys(&mut std::io::BufReader::new(std::fs::File::open(path)?))
        .map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid key")
        })
        .map(|mut keys| keys.drain(..).map(PrivateKey).collect())
}

// async fn flux_transfer(mut inbound: TcpStream, proxy_addr: String) ->
// Result<()> {     let mut outbound =
// tokio::net::TcpStream::connect(proxy_addr).await?;

//     let (mut ri, mut wi) = inbound.split();
//     let (mut ro, mut wo) = outbound.split();

//     let client_to_server = async {
//         io::copy(&mut ri, &mut wo).await?;
//         wo.shutdown().await
//     };

//     let server_to_client = async {
//         io::copy(&mut ro, &mut wi).await?;
//         wi.shutdown().await
//     };
//     tokio::try_join!(client_to_server, server_to_client)?;

//     return Ok(());
// }
