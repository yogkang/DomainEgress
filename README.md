# DomainEgress

Rust + Tauri 2 + Vue 3 macOS 本地代理客户端，提供 HTTP/HTTPS CONNECT 与 SOCKS5 TCP 代理，并支持白名单、黑名单两种准出策略。

本项目采用 [MIT License](LICENSE) 开源。项目自身代码适用 MIT 协议；第三方依赖及其许可证以各自项目声明为准。

内置白名单支持 GitHub Gist（`gist.github.com`、`gist.githubusercontent.com`）和 Gitee 在线代码片段（`gitee.com`）。

应用设置支持 GitHub Gist 和 Gitee 代码片段的规则推送与拉取。Token 仅用于当前请求，不保存到配置文件；拉取规则会先合并到草稿，保存配置后才生效。

## GitHub Release 与 Homebrew

正式 Release 安装包由 GitHub Actions 构建，使用 `gh` 跟踪构建及管理发布。推送版本 tag 后，GitHub Actions 会分别构建 macOS Apple Silicon（arm64）和 Intel（x86_64）安装包，并创建 Draft Release：

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
gh run list --workflow release.yml
gh run watch <run-id> --exit-status
# 安装包验证完成后发布草稿并设为 Latest
gh release edit vX.Y.Z --draft=false --latest
```

Homebrew Cask 模板位于 [`homebrew/Casks/domain-egress.rb`](homebrew/Casks/domain-egress.rb)。发布新版本后，将其复制到 `yogkang/homebrew-tap` 仓库的 `Casks/domain-egress.rb`，更新 `version` 和两个架构对应的 SHA-256。

### 使用 Homebrew 安装

当前通过个人 tap 分发，支持 Apple Silicon 和 Intel Mac：

```bash
brew tap yogkang/tap
brew install --cask domain-egress
```

验证安装：

```bash
brew info --cask domain-egress
open -a DomainEgress
```

升级和卸载：

```bash
brew upgrade --cask domain-egress
brew uninstall --cask domain-egress
```

如果尚未添加 tap，也可以直接执行：

```bash
brew install --cask yogkang/tap/domain-egress
```

当前发布流程尚未配置 Apple Developer ID 签名与公证；正式对外发布前应补充相关 GitHub Secrets。

### macOS 提示“应用已损坏，无法打开”

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

## 开发与构建

项目在根目录构建，使用 **Rust + Tauri 2 + Vue 3 + TypeScript + Vite**。
Vue 界面通过 Tauri IPC 调用 Rust。

### 启动

依赖：macOS 12+、Rust stable、Xcode Command Line Tools、Node.js 22.12+（或满足 Vite 7 要求的版本）。

```bash
./scripts/start-vue.sh
# 或：
npm ci
npm run desktop
```

本机开发需要安装应用时，可构建 `.app` 和 `.dmg`：

```bash
./scripts/install-vue.sh
```

产物位于 `src-tauri/target/release/bundle/`，包括 macOS 应用和 DMG 安装包。

开发启动会同时运行 Vite 和桌面窗口，支持界面热更新。
仅预览界面：在项目根目录运行 `npm run dev`，打开 `http://127.0.0.1:1420`。
浏览器预览明确标记为未连接核心，不提供代理启停、配置持久化或进程管理。

### 功能

- 代理概览：启停 HTTP/HTTPS CONNECT 与 SOCKS5 TCP，展示最近 8 个自然小时的策略放行次数。
- 访问控制：独立白名单/黑名单、批量添加、删除、清空确认、复制与排序。
- 访问日志：级别/关键词筛选、定时刷新、来源/方法/目标/参数/策略结果。
- 监听端口：macOS `lsof` 查询、搜索、进程运行时间、二次确认后发送 SIGTERM。
- 设置：监听 IP/端口、应用启动时自动启动代理、日志级别与保留天数。
- 托盘：显示窗口、启动、停止、退出；关闭窗口后继续驻留。

配置保存于 `~/Library/Application Support/DomainEgress/config.json`。
新版规则和设置先编辑草稿，点击“保存配置”才持久化并生效；监听地址/端口须先停止代理才能修改。
日志和趋势数据均在内存中，退出后清空，日志最多 2,000 条；保留天数在写入时执行清理。
“放行”表示策略允许请求，不代表上游连接或业务请求已成功。

### 构建与校验

```bash
cargo check --manifest-path src-tauri/Cargo.toml
npm run build
npm run package
```

独立桌面应用输出：`src-tauri/target/release/bundle/macos/DomainEgress.app`。
打包后的应用包含前端资源，无需启动 Node/Vite。此构建用于本机运行，未做 Apple 公证。

```bash
open "src-tauri/target/release/bundle/macos/DomainEgress.app"
```

面向使用者的安装和操作教程见：[使用说明](docs/使用说明.md)。

### 外观与程序员主题

在 Vue 3 版“应用设置 → 外观与主题”中选择跟随系统、浅色或暗黑模式。
提供 Forest、GitHub、Dracula、Nord、Monokai、Tokyo Night 六套配色，每套均支持浅色与暗黑。
切换即时生效，自动保存在当前应用的本地存储中，重启后恢复；无需保存代理配置。
桌面标题栏同步明暗模式；选择“跟随系统”时，系统外观变化会实时同步。
浏览器预览与桌面应用各自保存外观偏好。

## 本地静态 HTTP 服务

桌面应用侧栏的「静态 HTTP 服务」支持按端口创建临时静态资源服务：

- 输入根目录路径、选择目录，或将一个本地目录拖入配置页面，填写监听地址和端口后保存。
- 每个端口对应一个根目录；点击「启动」后通过页面显示的 HTTP 地址访问。默认监听 `127.0.0.1`，监听 `0.0.0.0` 可供局域网访问。
- 点击「修改目录」，重新选择或拖入目录并保存，后续请求立即使用新目录，无需重启服务。修改监听地址需先停止；使用不同端口会创建另一项服务。
- 支持 GET、HEAD、Range 和 MIME 类型识别；目录默认访问 `index.html`，不提供目录列表。路径穿越和指向根目录外的符号链接会被拒绝。
- 页面实时显示最近 1000 条客户端访问日志，同时输出至应用标准输出。日志仅保存在本次运行的内存中，可手动清空。
- 配置单独保存在应用数据目录的 `DomainEgress/static-http.json`；重启应用后保留配置，需手动启动服务。删除配置会停止该端口服务。

真实 HTTP 端到端验证（不使用单元测试，不修改用户配置）：

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example static-http-e2e -- /tmp/domain-egress-static-http-e2e
```

输出目录包含两个资源根目录、配置和 `report.json`（检查结果及真实客户端请求日志）。可重复执行，覆盖根目录热切换、多端口独立访问、中文路径、HEAD/Range、端口占用、路径安全、配置恢复、日志和启停删除。
