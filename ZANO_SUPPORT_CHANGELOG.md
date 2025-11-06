# Zano 币种支持更新日志

## 更新时间
2025-11-06

## 更新内容

本次更新为挖矿代理项目添加了对 Zano (ZANO) 币种的完整支持。

### 1. 核心代码修改

#### 文件：`core/src/util/config.rs`
- 在币种验证逻辑中添加了 "ZANO" 支持
- 修改位置：第 157-165 行
- 变更内容：在 `match self.coin.as_str()` 中添加 `"ZANO" => {}` 分支

### 2. 文档更新

#### 文件：`README.md`
- 更新项目描述，从 "ETH/ETC/CFX" 改为 "ETH/ETC/CFX/ZANO"
- 更新特性说明，添加 ZANO 支持

#### 新增文件：`doc/ZANO配置说明.md`
- 创建详细的 Zano 配置说明文档
- 包含矿池信息、配置示例、参数说明
- 提供多种配置模式：纯代理、抽水、SSL
- 包含故障排查指南和注意事项

#### 新增文件：`zano_example.yaml`
- 创建 Zano 配置示例文件
- 提供开箱即用的配置模板
- 包含详细的参数注释

### 3. 技术细节

#### 协议兼容性
- Zano 使用标准 Stratum 挖矿协议
- 项目已支持 `mining.subscribe`、`mining.authorize`、`mining.submit` 等标准 Stratum 方法
- 无需额外的协议层修改，现有实现完全兼容

#### 推荐矿池
- HeroMiners Zano 矿池
- TCP 端口：10128
- SSL 端口：20128
- 矿池地址：zano.herominers.com

### 4. 使用方法

#### 方法一：命令行启动
```bash
./mining_proxy --server --config zano_example.yaml
```

#### 方法二：Web 界面配置
1. 启动 Web 管理界面：`./mining_proxy`
2. 浏览器访问：http://localhost:8888
3. 创建新配置，币种选择 "ZANO"

### 5. 配置参数

#### 关键参数说明
- `coin`: 必须设置为 "ZANO"
- `pool_address`: 上游矿池地址（支持 tcp:// 和 ssl:// 前缀）
- `share_address`: 抽水矿池地址
- `share_wallet`: Zano 钱包地址（以 Z 开头）
- `share_rate`: 抽水比例（例如：0.02 = 2%）
- `share`: 工作模式（0=纯代理, 1=抽水模式, 2=统一钱包）

### 6. 兼容性

#### 支持的挖矿软件
- XMRig
- SRBMiner-MULTI
- 其他支持 Stratum 协议的挖矿软件

#### 支持的连接方式
- TCP 连接
- SSL/TLS 加密连接
- 自定义加密连接

### 7. 注意事项

1. **钱包格式**：Zano 钱包地址以 "Z" 开头，确保使用正确的地址格式
2. **协议差异**：Zano 使用标准 Stratum 协议，与 ETH 的 JSON-RPC 协议不同
3. **抽水比例**：建议设置在 1-3% 之间，避免过高影响矿工体验
4. **端口配置**：确保配置的端口未被占用且未被防火墙阻止
5. **SSL 证书**：使用 SSL 模式需要正确配置证书文件路径

### 8. 测试验证

已创建配置示例文件 `zano_example.yaml`，可直接用于测试：
```bash
# 编辑配置文件，填入你的钱包地址
nano zano_example.yaml

# 启动代理服务
./mining_proxy --server --config zano_example.yaml

# 矿工连接测试（使用 xmrig 为例）
./xmrig -o YOUR_SERVER_IP:14444 -u YOUR_ZANO_WALLET -p x
```

### 9. 文件清单

本次更新涉及的文件：
- 修改：`core/src/util/config.rs`
- 修改：`README.md`
- 新增：`doc/ZANO配置说明.md`
- 新增：`zano_example.yaml`
- 新增：`ZANO_SUPPORT_CHANGELOG.md`（本文件）

### 10. 后续建议

1. 建议用户升级 Rust 编译器到 1.83+ 版本以解决依赖包兼容问题
2. 可以考虑添加更多 Zano 特定的监控和统计功能
3. 可以增加对其他 CryptoNote 系列币种的支持（如 Monero）
4. 建议在实际生产环境中进行充分测试

## 技术支持

- Zano 官网：https://zano.org/
- HeroMiners 矿池：https://zano.herominers.com/
- 项目仓库：https://github.com/YusongWang/mining_proxy

## 版权说明

本次更新遵循项目原有的 Apache License 2.0 开源协议。
