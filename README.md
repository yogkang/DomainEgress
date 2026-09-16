# DomainEgress

Rust + Tauri 2 + Vue 3 macOS 本地代理客户端，提供 HTTP/HTTPS CONNECT 与 SOCKS5 TCP 代理，并支持白名单、黑名单两种准出策略。

本项目采用 [MIT License](LICENSE) 开源。项目自身代码适用 MIT 协议；第三方依赖及其许可证以各自项目声明为准。

## Vue 3 桌面版

新入口位于 `vue3/`，使用 **Rust + Tauri 2 + Vue 3 + TypeScript + Vite**。
Vue 界面通过 Tauri IPC 调用 Rust。

### 启动

依赖：macOS 12+、Rust stable、Xcode Command Line Tools、Node.js 22.12+（或满足 Vite 7 要求的版本）。

```bash
./scripts/start-vue.sh
# 或：
cd vue3
npm ci
npm run desktop
```

直接打包 `.app` 和 `.dmg` 安装包：

```bash
./scripts/install-vue.sh
```

产物位于 `vue3/src-tauri/target/release/bundle/`，包括 macOS 应用和 DMG 安装包。

开发启动会同时运行 Vite 和桌面窗口，支持界面热更新。
仅预览界面：在 `vue3` 中运行 `npm run dev`，打开 `http://127.0.0.1:1420`。
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
cargo test --manifest-path vue3/src-tauri/Cargo.toml
cd vue3
npm run build
npm run package
```

独立桌面应用输出：`vue3/src-tauri/target/release/bundle/macos/DomainEgress Vue.app`。
打包后的应用包含前端资源，无需启动 Node/Vite。此构建用于本机运行，未做 Apple 公证。

```bash
open "vue3/src-tauri/target/release/bundle/macos/DomainEgress Vue.app"
```

面向使用者的安装和操作教程见：[vue3/README.md](vue3/README.md)。

### 外观与程序员主题

在 Vue 3 版“应用设置 → 外观与主题”中选择跟随系统、浅色或暗黑模式。
提供 Forest、GitHub、Dracula、Nord、Monokai、Tokyo Night 六套配色，每套均支持浅色与暗黑。
切换即时生效，自动保存在当前应用的本地存储中，重启后恢复；无需保存代理配置。
桌面标题栏同步明暗模式；选择“跟随系统”时，系统外观变化会实时同步。
浏览器预览与桌面应用各自保存外观偏好。
