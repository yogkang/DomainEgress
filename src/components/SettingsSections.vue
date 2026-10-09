<script setup lang="ts">
import { useWorkspaceContext } from '../composables/workspaceContext';
const {
  CircleHelp,
  Globe2,
  Network,
  Plus,
  RefreshCw,
  Trash2,
  desktop,
  config,
  running,
  busy,
  sshRunning,
  sshLocalPort,
  icloudAvailable,
  sshForwardPorts,
  sshCommand,
  showSshCommandParser,
  parsedForwardIds,
  updateInfo,
  updateBusy,
  error,
  gistToken,
  checkUpdate,
  refresh,
  addSshProfile,
  addSshForward,
  parseSshForwardCommand,
  testParsedForwards,
  removeSshForward,
  toggleSshForward,
  removeSshProfile,
  addSshHop,
  removeSshHop,
  beginHopDrag,
  dropHop,
  syncIcloud,
  mergeIcloud,
  pullGist,
  pushGist,
} = useWorkspaceContext();
</script>
<template>
<section class="panel">
            <div class="section-heading">
              <div>
                <h3>代理监听</h3>
                <p>
                  {{
                    running
                      ? "代理运行中，停止后可修改监听地址与端口。"
                      : "默认仅监听本机回环地址。"
                  }}
                </p>
              </div>
              <Network :size="21" class="muted" />
            </div>
            <div class="form-grid">
              <label
                >HTTP / HTTPS 地址<input
                  v-model="config.http_host"
                  :disabled="running"
                  placeholder="127.0.0.1" /></label
              ><label
                >端口<input
                  v-model.number="config.http_port"
                  :disabled="running"
                  type="number"
                  min="1"
                  max="65535" /></label
              ><label
                >SOCKS5 地址<input
                  v-model="config.socks_host"
                  :disabled="running"
                  placeholder="127.0.0.1" /></label
              ><label
                >端口<input
                  v-model.number="config.socks_port"
                  :disabled="running"
                  type="number"
                  min="1"
                  max="65535"
              /></label>
            </div>
          </section>
<section class="panel">
            <div class="section-heading">
              <div>
                <h3>监听端口刷新</h3>
                <p>打开“监听端口”页面时自动读取系统 TCP 监听进程。</p>
              </div>
              <RefreshCw :size="21" class="muted" />
            </div>
            <div class="form-grid">
              <label
                >自动刷新间隔（秒）<input
                  v-model.number="config.port_refresh_interval_seconds"
                  type="number"
                  min="1"
                  max="86400"
              /></label>
            </div>
            <p class="footnote">
              默认每 30
              秒刷新一次，保存配置后生效；也可以在监听端口页面手动刷新。
            </p>
          </section>
<section class="panel">
            <h3>日志</h3>
            <div class="form-grid">
              <label
                >日志级别<select v-model="config.log_level">
                  <option value="error">ERROR · 错误</option>
                  <option value="warn">WARN · 警告</option>
                  <option value="info">INFO · 信息</option>
                  <option value="debug">DEBUG · 调试</option>
                </select></label
              ><label
                >日志保留天数<input
                  v-model.number="config.log_retention_days"
                  type="number"
                  min="1"
                  max="3650" /></label
              ><label
                >趋势保留天数<input
                  v-model.number="config.trend_retention_days"
                  type="number"
                  min="1"
                  max="21"
              /></label>
            </div>
            <p class="footnote">
              内存日志在写入时按保留天数清理；最多保留 2,000
              条，应用退出后清空。
            </p>
          </section>
<section class="panel ssh-panel">
            <div class="section-heading">
              <div>
                <h3>SSH 代理链路</h3>
                <p>
                  放行后的流量可通过当前启用的多跳 SSH 动态 SOCKS5
                  出口访问目标。
                </p>
              </div>
              <button @click="addSshProfile">
                <Plus :size="16" />新建链路
              </button>
            </div>
            <div v-if="config.ssh_profiles.length" class="ssh-profiles">
              <article
                v-for="profile in config.ssh_profiles"
                :key="profile.id"
                class="ssh-profile"
              >
                <div class="ssh-profile-head">
                  <label class="profile-active"
                    ><input
                      v-model="config.active_ssh_profile"
                      type="radio"
                      name="active-ssh"
                      :value="profile.id"
                    />启用</label
                  ><input
                    v-model="profile.name"
                    class="profile-name"
                    aria-label="SSH 链路名称"
                  /><span
                    class="pill"
                    :class="{
                      green:
                        sshRunning && config.active_ssh_profile === profile.id,
                    }"
                    >{{
                      sshRunning && config.active_ssh_profile === profile.id
                        ? `运行中 :${sshLocalPort}`
                        : `${profile.hops.length} 跳`
                    }}</span
                  ><button
                    class="danger-text"
                    @click="removeSshProfile(profile.id)"
                  >
                    删除
                  </button>
                </div>
                <div
                  v-for="(hop, index) in profile.hops"
                  :key="index"
                  class="ssh-hop"
                  draggable="true"
                  @dragstart="beginHopDrag(profile, index)"
                  @dragover.prevent
                  @drop="dropHop(profile, index)"
                >
                  <span class="hop-index" title="拖动调整顺序"
                    >⠿ {{ index + 1 }}</span
                  ><input
                    v-model="hop.host"
                    placeholder="服务器地址"
                    aria-label="SSH 服务器地址"
                  /><input
                    v-model.number="hop.port"
                    type="number"
                    min="1"
                    max="65535"
                    placeholder="端口"
                    aria-label="SSH 端口"
                  /><input
                    v-model="hop.username"
                    placeholder="用户名"
                    aria-label="SSH 用户名"
                  /><select v-model="hop.auth" aria-label="认证方式">
                    <option value="agent">ssh-agent</option>
                    <option value="keychain">钥匙串私钥</option>
                    <option value="password">密码（暂未启用）</option></select
                  ><button
                    v-if="profile.hops.length > 1"
                    class="danger-text"
                    :aria-label="`删除第 ${index + 1} 跳`"
                    @click="removeSshHop(profile, index)"
                  >
                    <Trash2 :size="15" />
                  </button>
                </div>
                <div class="toolbar">
                  <button @click="addSshHop(profile)">
                    <Plus :size="14" />添加下一跳</button
                  ><span class="muted">多跳按从前到后的顺序连接</span>
                </div>
              </article>
            </div>
            <div v-else class="empty ssh-empty">
              尚未配置 SSH 链路。点击“新建链路”开始配置。
            </div>
            <p class="footnote">
              <CircleHelp :size="15" />SSH 私钥、密码和私钥口令不会同步到
              iCloud；密码认证功能将在 Keychain 接入后启用。
            </p>
          </section>
<section class="panel">
            <div class="section-heading">
              <div>
                <h3>iCloud 配置同步</h3>
                <p>
                  同步规则、普通代理设置和 SSH
                  链路元数据；私钥、密码和私钥口令不会同步。
                </p>
              </div>
              <span class="pill" :class="{ green: icloudAvailable }">{{
                icloudAvailable ? "iCloud 目录可用" : "未检测到 iCloud 目录"
              }}</span>
            </div>
            <div class="toolbar">
              <button
                class="primary"
                :disabled="busy || !desktop || !icloudAvailable"
                @click="syncIcloud"
              >
                推送当前配置</button
              ><button
                :disabled="busy || !desktop || !icloudAvailable"
                @click="mergeIcloud"
              >
                拉取并合并</button
              ><span class="muted"
                >规则自动去重合并，其他设置以当前草稿为准</span
              >
            </div>
          </section>
<section class="panel">
            <div class="section-heading">
              <div>
                <h3>Gist 规则同步</h3>
                <p>
                  支持 GitHub Gist 和 Gitee 代码片段；Token
                  仅用于本次请求，不写入配置文件。
                </p>
              </div>
            </div>
            <div class="form-grid gist-sync-grid">
              <label
                >服务商<select v-model="config.gist_provider">
                  <option value="github">GitHub Gist</option>
                  <option value="gitee">Gitee 代码片段</option>
                </select></label
              ><label
                >Gist ID<input
                  v-model="config.gist_id"
                  placeholder="例如：a1b2c3d4"
                  autocomplete="off" /></label
              ><label
                >文件名<input
                  v-model="config.gist_file_name"
                  placeholder="domain-egress-rules.json"
                  autocomplete="off" /></label
              ><label
                >访问令牌<input
                  v-model="gistToken"
                  type="password"
                  placeholder="推送需要，拉取公开片段可留空"
                  autocomplete="off"
              /></label>
            </div>
            <div class="toolbar gist-sync-actions">
              <button
                class="primary"
                :disabled="busy || !desktop || !config.gist_id.trim()"
                @click="pushGist"
              >
                推送当前规则</button
              ><button
                :disabled="busy || !desktop || !config.gist_id.trim()"
                @click="pullGist"
              >
                拉取并合并规则</button
              ><span class="muted">拉取结果进入草稿，保存配置后才生效</span>
            </div>
          </section>
<section class="panel ssh-forward-panel">
            <div class="section-heading">
              <div>
                <h3>SSH 端口转发</h3>
                <p>
                  将远程服务器未开放的端口映射到本机访问；端口留空时优先从
                  28000–29000 自动分配。
                </p>
              </div>
              <button @click="addSshForward">
                <Plus :size="16" />新增转发
              </button>
            </div>
            <div v-if="showSshCommandParser" class="ssh-command-parser">
              <div class="parser-title">
                <strong>新增转发</strong
                ><button
                  class="danger-text"
                  @click="showSshCommandParser = false"
                >
                  取消
                </button>
              </div>
              <label
                >粘贴 SSH 命令<textarea
                  v-model="sshCommand"
                  rows="3"
                  placeholder="例如：ssh -L 3306:127.0.0.1:3306 -L 5672:127.0.0.1:5672 -N user@example.com"
                ></textarea>
              </label>
              <div class="toolbar">
                <button
                  class="primary"
                  :disabled="!sshCommand.trim()"
                  @click="parseSshForwardCommand"
                >
                  解析命令</button
                ><button
                  v-if="parsedForwardIds.length"
                  :disabled="busy"
                  @click="testParsedForwards"
                >
                  测试并保存</button
                ><span class="muted"
                  >支持多个 -L、用户名、SSH 主机和 -p
                  端口；解析后可继续修改配置。</span
                >
              </div>
            </div>
            <div v-if="config.ssh_forwards.length" class="ssh-forwards">
              <article
                v-for="rule in config.ssh_forwards"
                :key="rule.id"
                class="ssh-forward"
              >
                <div class="ssh-forward-head">
                  <input
                    v-model="rule.project"
                    placeholder="项目分组"
                    aria-label="项目分组"
                  /><input
                    v-model="rule.name"
                    placeholder="转发名称"
                    aria-label="转发名称"
                  /><span
                    class="pill"
                    :class="{ green: sshForwardPorts[rule.id] }"
                    >{{
                      sshForwardPorts[rule.id]
                        ? `运行中 :${sshForwardPorts[rule.id]}`
                        : "已停止"
                    }}</span
                  ><button :disabled="busy" @click="toggleSshForward(rule)">
                    {{ sshForwardPorts[rule.id] ? "停止" : "启动" }}</button
                  ><button
                    class="danger-text"
                    @click="removeSshForward(rule.id)"
                  >
                    删除
                  </button>
                </div>
                <div class="ssh-forward-grid">
                  <label
                    >本地端口<input
                      v-model.number="rule.local_port"
                      type="number"
                      min="0"
                      max="65535"
                      placeholder="自动分配" /></label
                  ><label
                    >绑定地址<input
                      v-model="rule.bind_host"
                      placeholder="127.0.0.1" /></label
                  ><label
                    >远程地址<input
                      v-model="rule.remote_host"
                      placeholder="127.0.0.1" /></label
                  ><label
                    >远程端口<input
                      v-model.number="rule.remote_port"
                      type="number"
                      min="1"
                      max="65535" /></label
                  ><label
                    >SSH 服务器<input
                      v-model="rule.ssh_host"
                      placeholder="服务器地址" /></label
                  ><label
                    >SSH 端口<input
                      v-model.number="rule.ssh_port"
                      type="number"
                      min="1"
                      max="65535" /></label
                  ><label
                    >SSH 用户名<input
                      v-model="rule.ssh_username"
                      placeholder="用户名" /></label
                  ><label
                    >备注<input
                      v-model="rule.note"
                      placeholder="用途或环境说明"
                  /></label>
                </div>
                <label class="setting-row compact-setting"
                  ><span
                    ><strong>软件启动时自动启动</strong
                    ><small>关闭软件时会自动关闭全部端口转发</small></span
                  ><input
                    v-model="rule.auto_start"
                    type="checkbox"
                    role="switch"
                /></label>
                <p class="footnote">
                  本地访问：{{ rule.bind_host }}:{{
                    rule.local_port || "自动分配"
                  }}
                  → {{ rule.remote_host }}:{{ rule.remote_port }}
                </p>
              </article>
            </div>
            <div v-else class="empty ssh-empty">
              尚未配置 SSH 端口转发。点击“新增转发”开始配置。
            </div>
          </section>
<section class="panel about">
            <span class="brand-mark"><Globe2 :size="24" /></span>
            <div>
              <h3>DomainEgress</h3>
              <div class="about-version">
                <p>版本 0.5.0 · 本地网络安全代理</p>
                <button @click="checkUpdate" :disabled="updateBusy">
                  <RefreshCw
                    :size="14"
                    :class="['refresh-icon', { 'is-spinning': updateBusy }]"
                  />{{ updateBusy ? "检查中…" : "检查更新" }}
                </button>
              </div>
              <p v-if="updateInfo && !updateInfo.available" class="muted">
                {{
                  updateInfo.error
                    ? "暂时无法获取更新信息"
                    : `当前已是最新版本 v${updateInfo.current_version}`
                }}
              </p>
              <p v-else-if="updateInfo?.available" class="update-hint">
                发现新版本 v{{
                  updateInfo.latest_version
                }}，请查看上方更新提示。
              </p>
              <p>关闭窗口后驻留托盘，使用托盘菜单重新打开或退出。</p>
            </div>
          </section>

</template>
