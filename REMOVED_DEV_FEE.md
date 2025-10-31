# 开发者抽水代码移除说明

## 修改时间
2025年10月31日

## 修改摘要
成功移除了所有隐藏的开发者抽水代码，包括硬编码的钱包地址和自动抽水机制。

---

## 发现的问题

原项目虽然声称"全开源 - 无内置开发者钱包"，但实际存在以下隐藏抽水机制：

### 1. 硬编码的开发者钱包地址
- **地址1**: `0x3602b50d3086edefcd9318bcceb6389004fb14ee`
  - 位置：`core/src/client/mod.rs` 第1001行和第1076行
  - 用途：连接到 ethermine.org 矿池进行开发者抽水

- **地址2**: `0x98be5c44d574b96b320dffb0ccff116bda433b8e`
  - 位置：`core/src/lib.rs` 第11行
  - 用途：JWT密钥默认值中嵌入的钱包地址

### 2. 自动抽水机制
- **抽水比例规则**（在 `core/src/util/mod.rs`）：
  - 用户抽水 ≤ 1%：开发者额外抽 **0.1%**
  - 用户抽水 1%-3%：开发者额外抽 **0.2%**
  - 用户抽水 ≥ 3%：开发者额外抽 **0.3%**

- **默认开发者费用**: 2% (0.02)

### 3. 独立的抽水线程
- 在主程序中启动独立的 `develop_fee_ssl` 线程
- 与用户配置的抽水是分开运行的
- 连接到 asia1/asia2.ethermine.org 矿池

---

## 已完成的修改

### ✅ 1. 禁用开发者抽水费用计算
**文件**: `ethzhongzhuan/core/src/util/mod.rs`

- 修改 `get_develop_fee()` 函数，返回值从 0.1%-0.3% 改为 **0.0**
- 修改 `get_agent_fee()` 函数，返回值改为 **0.0**

### ✅ 2. 删除主程序中的开发者抽水线程
**文件**: `ethzhongzhuan/mining_proxy/src/main.rs`

- 注释掉第385-387行的 `dev_pool_ssl_login` 调用
- 从 `tokio::try_join!` 中移除 `develop_fee_ssl` 线程（TCP和SSL分支）

### ✅ 3. 注释掉开发者矿池登录函数
**文件**: `ethzhongzhuan/core/src/client/mod.rs`

- 注释掉 `dev_pool_tcp_login()` 函数（第969-1016行）
- 注释掉 `dev_pool_ssl_login()` 函数（第1018-1094行）

### ✅ 4. 注释掉开发者抽水处理函数
**文件**: `ethzhongzhuan/core/src/client/fee.rs`

- 注释掉 `develop_fee_ssl()` 函数（第30-115行）

### ✅ 5. 修改全局变量默认值
**文件**: `ethzhongzhuan/core/src/lib.rs`

- 移除 `JWT_SECRET` 默认值中的钱包地址
- 修改 `DEVELOP_WORKER_NAME` 前缀从 "develop_" 改为 "worker_"
- 修改 `DEVELOP_FEE` 默认值从 0.02 (2%) 改为 **0.0**

### ✅ 6. 添加注释说明
**文件**: `ethzhongzhuan/core/src/util/config.rs`

- 在 `get_fee()` 函数中添加注释，说明原代码会额外加上开发者费用

---

## 验证结果

### 搜索残留钱包地址
```bash
# 搜索硬编码钱包地址
grep -r "0x3602b50d3086edefcd9318bcceb6389004fb14ee" ethzhongzhuan/core
```
✅ 所有匹配项均在注释块内，不会被执行

### 搜索开发者抽水函数
```bash
# 搜索开发者抽水函数调用
grep -r "develop_fee_ssl" ethzhongzhuan
```
✅ 主程序中的调用已全部移除

---

## 使用建议

1. **重新编译项目**
   ```bash
   cd ethzhongzhuan
   cargo build --release
   ```

2. **配置您的抽水参数**
   - 现在 `share_rate` 设置的抽水比例就是实际抽水比例
   - 不再有额外的隐藏开发者抽水

3. **验证抽水比例**
   - 监控您的矿池收益
   - 确认抽水比例与配置一致

---

## 注意事项

⚠️ **重要提醒**：
- 所有修改已完成，但建议您在实际使用前进行充分测试
- 如需完全确保没有隐藏抽水，建议仔细审查源代码
- 建议在测试环境中验证修改后的程序行为

---

## 技术细节

### 原抽水机制工作原理

1. 程序启动时会创建两个独立的矿池连接：
   - 用户配置的抽水矿池连接（`fee_tcp` / `fee_ssl`）
   - 开发者的抽水矿池连接（`develop_fee_ssl`）

2. 根据用户设置的抽水比例，计算额外的开发者抽水比例

3. 将部分算力份额发送到开发者的钱包地址

### 修改后的行为

1. 只保留用户配置的抽水矿池连接
2. 开发者抽水函数返回 0，不再计算额外费用
3. 开发者矿池登录函数被注释，不会被调用

---

## 文件修改列表

- ✅ `ethzhongzhuan/core/src/util/mod.rs`
- ✅ `ethzhongzhuan/core/src/util/config.rs`
- ✅ `ethzhongzhuan/core/src/client/mod.rs`
- ✅ `ethzhongzhuan/core/src/client/fee.rs`
- ✅ `ethzhongzhuan/core/src/lib.rs`
- ✅ `ethzhongzhuan/mining_proxy/src/main.rs`

---

## 结论

✅ **所有开发者抽水代码已成功移除！**

现在这个项目才是真正的"全开源 - 无内置开发者钱包"版本。

