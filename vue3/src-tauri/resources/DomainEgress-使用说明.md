# DomainEgress 使用说明

DomainEgress 是一款 macOS 本地网络访问控制代理，提供 HTTP/HTTPS 和 SOCKS5 代理、域名白名单/黑名单、访问日志、SSH 多跳出口和 iCloud 配置同步。

## 第一次使用

默认代理地址：

```text
HTTP/HTTPS：127.0.0.1:19876
SOCKS5：127.0.0.1:16789
```

在浏览器或其他应用中配置代理后，回到“代理概览”点击“启动代理”。

## macOS 提示“应用已损坏，无法打开”

当前安装包尚未完成 Apple 公证。若 macOS 显示“`DomainEgress.app` 已损坏，无法打开。你应该将它移到废纸篓”的提示，请先确认应用来自可信的 Release 或安装包，再任选以下方式处理：

1. 在终端执行以下命令，输入管理员密码后，前往“设置 → 隐私与安全性 → 安全性”，选择“任何来源”：

   ```bash
   sudo spctl --master-disable
   ```

2. 或者仅移除该应用的下载隔离属性。将命令中的应用名替换为实际名称：

   ```bash
   sudo xattr -rd com.apple.quarantine /Applications/xxx.app
   ```

   例如：

   ```bash
   sudo xattr -rd com.apple.quarantine /Applications/DomainEgress.app
   ```

## 访问控制

白名单模式只允许匹配规则的目标，白名单为空时拒绝全部访问。黑名单模式默认允许访问，只拦截命中规则的目标。

裸域名匹配自身及所有层级子域名：

```text
aliyun.com
```

匹配 `aliyun.com`、`tongyi.aliyun.com` 和 `lingma-api.tongyi.aliyun.com`。

通配符只匹配一级子域名：

```text
*.aliyun.com
```

匹配 `api.aliyun.com`，但不匹配 `aliyun.com` 或 `a.b.aliyun.com`。

访问日志中的目标支持右键添加规则，并可同时选择多个候选规则。规则先进入草稿，点击“保存配置”后才生效。

## SSH 多跳代理

SSH 链路可以将放行流量继续转发：

```text
本机代理 → SSH A → SSH B → 目标服务器
```

使用步骤：

1. 打开“应用设置 → SSH 代理链路”。
2. 点击“新建链路”，填写地址、端口和用户名。
3. 点击“添加下一跳”增加跳板。
4. 拖动节点左侧的 `⠿` 调整顺序。
5. 选择认证方式并启用链路。
6. 保存配置后启动代理。

SSH 私钥、密码和私钥口令不会同步到 iCloud。

## iCloud 配置同步

iCloud 只同步白名单、黑名单、普通代理设置和 SSH 链路元数据，不同步敏感凭据。

1. 确认 Mac 已登录 iCloud 并启用 iCloud Drive。
2. 打开“应用设置 → iCloud 配置同步”。
3. 点击“推送当前配置”上传配置。
4. 在另一台 Mac 点击“拉取并合并”。
5. 检查草稿并点击“保存配置”。

规则会自动去重合并，普通设置以当前草稿为准。

## 访问日志

日志包含时间、来源、方法、目标、参数和放行/拦截结果。日志最多保留 2,000 条，保存在内存中，退出应用后清空。“放行”只表示策略允许，不代表目标服务器一定访问成功。

## 常见问题

- 启动失败：检查“监听端口”，确认 19876 或 16789 没有被其他程序占用。
- 端口无法修改：先停止代理，再修改并保存。
- SSH 失败：检查地址、端口、用户名、ssh-agent 私钥和跳板顺序。
- 浏览器预览不能启停代理：预览页面未连接 Rust 核心，请使用已安装的 DomainEgress 应用。
