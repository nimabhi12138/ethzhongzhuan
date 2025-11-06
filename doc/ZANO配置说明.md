# Zano 币种配置说明

## 简介

本项目现已支持 Zano (ZANO) 币种的挖矿代理和抽水功能。Zano 是一个基于 CryptoNote 的隐私币，使用标准的 Stratum 挖矿协议。

## 矿池信息

推荐使用 HeroMiners 矿池：
- 官网：https://zano.herominers.com/
- 矿池地址示例：
  - TCP: `tcp://zano.herominers.com:10128` (TCP端口)
  - SSL: `ssl://zano.herominers.com:20128` (SSL端口)

## 配置示例

### 基本配置（纯代理模式）

```yaml
coin: ZANO
name: zano-proxy-1
log_level: DEBUG
tcp_port: 14444
ssl_port: 8443
encrypt_port: 0
pool_address:
  - tcp://zano.herominers.com:10128
share_address:
  - tcp://zano.herominers.com:10128
share_wallet: ""
share_name: zano_worker_01
share_rate: 0.0
share: 0
share_alg: 0
hash_rate: 100
pem_path: ./cert.pem
key_path: ./key.pem
```

### 抽水模式配置

```yaml
coin: ZANO
name: zano-proxy-fee
log_level: DEBUG
tcp_port: 14444
ssl_port: 8443
encrypt_port: 0
pool_address:
  - tcp://zano.herominers.com:10128
share_address:
  - tcp://zano.herominers.com:10128
share_wallet: Zxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx  # 你的 Zano 钱包地址
share_name: zano_fee_worker
share_rate: 0.02  # 2% 抽水比例
share: 1  # 1=抽水模式, 0=纯代理模式, 2=统一钱包模式
share_alg: 0
hash_rate: 100
pem_path: ./cert.pem
key_path: ./key.pem
```

### SSL 模式配置

```yaml
coin: ZANO
name: zano-proxy-ssl
log_level: DEBUG
tcp_port: 14444
ssl_port: 8443
encrypt_port: 0
pool_address:
  - ssl://zano.herominers.com:20128  # 使用 SSL 端口
share_address:
  - ssl://zano.herominers.com:20128
share_wallet: Zxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
share_name: zano_ssl_worker
share_rate: 0.015  # 1.5% 抽水比例
share: 1
share_alg: 0
hash_rate: 100
pem_path: ./cert.pem
key_path: ./key.pem
```

## 配置参数说明

- `coin`: 币种代码，填写 `ZANO`
- `name`: 代理服务器名称，用于标识不同的代理实例
- `tcp_port`: 本地 TCP 监听端口
- `ssl_port`: 本地 SSL 监听端口
- `encrypt_port`: 本地加密端口（可选）
- `pool_address`: 上游矿池地址（矿工连接的目标矿池）
- `share_address`: 抽水矿池地址
- `share_wallet`: 抽水钱包地址
- `share_name`: 抽水矿工名称
- `share_rate`: 抽水比例（0.02 = 2%）
- `share`: 模式选择
  - 0 = 纯代理模式（不抽水）
  - 1 = 抽水模式
  - 2 = 统一钱包模式
- `share_alg`: 抽水算法选择（0=默认）

## 启动方法

### 使用配置文件启动

1. 创建配置文件 `zano.yaml`
2. 运行命令：
```bash
./mining_proxy --server --config zano.yaml
```

### 使用 Web 界面配置

1. 启动 Web 管理界面：
```bash
./mining_proxy
```

2. 浏览器访问 `http://localhost:8888`（默认端口）

3. 在 Web 界面中创建新的代理配置：
   - 币种选择：ZANO
   - 填写矿池地址
   - 设置抽水比例和钱包地址
   - 保存并启动

## 矿工连接

矿工软件连接到你的代理服务器：

```bash
# 示例：使用 xmrig 连接
./xmrig -o YOUR_SERVER_IP:14444 -u YOUR_ZANO_WALLET -p x
```

## 支持的挖矿软件

Zano 支持大多数 Stratum 兼容的挖矿软件：
- XMRig
- SRBMiner-MULTI
- 其他支持 Stratum 协议的挖矿软件

## 注意事项

1. **钱包地址格式**：确保使用正确的 Zano 钱包地址格式（以 Z 开头）
2. **矿池连接**：Zano 使用标准 Stratum 协议，与 ETH 等币种的协议不同
3. **端口配置**：确保配置的端口未被占用
4. **SSL 证书**：如果使用 SSL 模式，需要正确配置 `pem_path` 和 `key_path`
5. **抽水比例**：建议抽水比例不要设置过高，一般在 1-3% 之间

## 故障排查

### 连接失败
- 检查矿池地址是否正确
- 确认网络连接正常
- 验证端口是否被防火墙阻止

### 抽水不生效
- 确认 `share` 参数设置为 1
- 检查 `share_wallet` 是否填写正确
- 验证 `share_rate` 大于 0

### 日志查看
日志文件位于 `./logs/` 目录下，查看日志以获取详细的错误信息。

## 其他资源

- Zano 官网：https://zano.org/
- HeroMiners Zano 矿池：https://zano.herominers.com/
- 项目 GitHub：https://github.com/YusongWang/mining_proxy
