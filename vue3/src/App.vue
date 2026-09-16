<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { Activity, ArrowUpRight, Check, ChevronRight, CircleHelp, Copy, Download, Globe2, ListFilter, Network, Play, Plus, RefreshCw, Search, Settings2, ShieldCheck, Square, Trash2, Upload, X } from 'lucide-vue-next'
import { appearance, theme, themes, appearanceError, setAppearance, setTheme } from './theme'
import { call, defaults, desktop, type Config, type LogEntry, type PortRow, type Snapshot, type SshProfile } from './api'
const tabs = [{ id: 'overview', title: '代理概览', icon: Activity }, { id: 'rules', title: '访问控制', icon: ShieldCheck }, { id: 'logs', title: '访问日志', icon: ListFilter }, { id: 'ports', title: '监听端口', icon: Network }, { id: 'settings', title: '应用设置', icon: Settings2 }]
const tab = ref('overview'), config = ref<Config>(structuredClone(defaults)), saved = ref<Config>(structuredClone(defaults))
const running = ref(false), logs = ref<LogEntry[]>([]), traffic = ref<number[]>([]), busy = ref(false), connected = ref(!desktop), initialized = ref(false), sshRunning = ref(false), sshLocalPort = ref<number | null>(null), icloudAvailable = ref(false)
const notice = ref(''), error = ref(false), draft = ref(''), search = ref(''), logLevel = ref('all'), logOutcome = ref('all'), ruleSort = ref('name'), ruleSearch = ref(''), trendRange = ref(60), trendInterval = ref(60), autoScrollLogs = ref(true), trendStart = ref(''), trendEnd = ref(''), undoRule = ref<{ rules: string[]; mode: 'whitelist' | 'blacklist' } | null>(null)
const ports = ref<PortRow[]>([]), portBusy = ref(false), portLoaded = ref(false), portSearch = ref(''), confirmAction = ref<'clear' | 'save-mode' | PortRow | null>(null), pendingSave = ref<Config | null>(null), contextMenu = ref<{ target: string; x: number; y: number; candidates: string[] } | null>(null), selectedTargets = ref<string[]>([]), sshSecrets = ref<Record<string, string>>({}), draggedHop = ref<{ profile: SshProfile; index: number } | null>(null)
const logPanel = ref<HTMLElement | null>(null)
const dirty = computed(() => JSON.stringify(config.value) !== JSON.stringify(saved.value))
const modeName = computed(() => config.value.access_mode === 'whitelist' ? '白名单' : '黑名单')
const activeRules = computed(() => config.value[config.value.access_mode])
const timestamps = computed(() => config.value[config.value.access_mode === 'whitelist' ? 'whitelist_added_at' : 'blacklist_added_at'])
const orderedRules = computed(() => [...activeRules.value].filter(r => r.includes(ruleSearch.value.trim().toLowerCase())).sort((a, b) => ruleSort.value === 'name' ? a.localeCompare(b) : (timestamps.value[b] || 0) - (timestamps.value[a] || 0)))
const filteredLogs = computed(() => logs.value.filter(l => (logLevel.value === 'all' || logLevel.value === l.level) && (logOutcome.value === 'all' || logOutcome.value === l.outcome) && Object.values(l).join(' ').toLowerCase().includes(search.value.toLowerCase())).reverse())
const range = computed(() => { const end = Date.now() / 1000; return { start: end - trendRange.value * 60, end } })
const blockedLogs = computed(() => logs.value.filter(l => l.outcome === '拦截' && l.timestamp >= range.value.start && l.timestamp <= range.value.end))
const blockedDomains = computed(() => { const grouped = new Map<string, LogEntry & { count: number }>(); for (const log of blockedLogs.value) { const key = `${log.target}\u0000${log.source}`; const item = grouped.get(key); if (item) item.count += 1; else grouped.set(key, { ...log, count: 1 }) } return [...grouped.values()].sort((a, b) => b.count - a.count || b.timestamp - a.timestamp) })
const filteredPorts = computed(() => ports.value.filter(p => Object.values(p).join(' ').toLowerCase().includes(portSearch.value.toLowerCase())))
const buckets = computed(() => { const step = trendInterval.value; const start = Math.floor(range.value.start / step) * step; const end = Math.ceil(range.value.end / step) * step; const count = Math.min(1440, Math.max(1, Math.ceil((end - start) / step))); return Array.from({ length: count }, (_, i) => { const bucket = start + i * step; return { label: new Date(bucket * 1000).toLocaleString('zh-CN', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' }), count: traffic.value.filter(t => t >= bucket && t < bucket + step && t >= range.value.start && t <= range.value.end).length } }) })
const maximum = computed(() => Math.max(1, ...buckets.value.map(b => b.count)))
const trendRangeLabel = computed(() => trendRange.value < 60 ? `最近 ${trendRange.value} 分钟` : `最近 ${trendRange.value / 60} 小时`)
const admitted = computed(() => buckets.value.reduce((n, b) => n + b.count, 0))
const blocked = computed(() => logs.value.filter(l => l.outcome === '拦截').length)
const date = (t: number) => t ? new Date(t * 1000).toLocaleString('zh-CN', { hour12: false }) : '历史规则'
const ruleDate = (t: number) => t ? date(t) : '内置规则'
function notify(message: string, failed = false) { notice.value = message; error.value = failed }
function localInput(t: number) { const d = new Date(t * 1000 - new Date().getTimezoneOffset() * 60000); return d.toISOString().slice(0, 16) }
function setTrend(value: number) { trendRange.value = value }
function exportRules() { const payload = { format: 'domain-egress-rules', version: 1, exported_at: new Date().toISOString(), access_mode: config.value.access_mode, whitelist: config.value.whitelist, blacklist: config.value.blacklist, whitelist_added_at: config.value.whitelist_added_at, blacklist_added_at: config.value.blacklist_added_at }; const a = document.createElement('a'); a.href = URL.createObjectURL(new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' })); a.download = 'domain-egress-rules.json'; a.click(); URL.revokeObjectURL(a.href); notify('规则已导出') }
function importRules(event: Event) { const input = event.target as HTMLInputElement; const file = input.files?.[0]; if (!file) return; const reader = new FileReader(); reader.onload = () => { try { const x = JSON.parse(String(reader.result)); if (x.format !== 'domain-egress-rules' || x.version !== 1 || !Array.isArray(x.whitelist) || !Array.isArray(x.blacklist)) throw new Error('文件格式不正确'); if (![...x.whitelist, ...x.blacklist].every((r: unknown) => typeof r === 'string' && r.trim() && r.length <= 253)) throw new Error('文件包含无效规则'); config.value = { ...config.value, access_mode: x.access_mode === 'blacklist' ? 'blacklist' : 'whitelist', whitelist: [...new Set(x.whitelist as string[])], blacklist: [...new Set(x.blacklist as string[])], whitelist_added_at: x.whitelist_added_at || {}, blacklist_added_at: x.blacklist_added_at || {} }; notify('规则已导入草稿，请保存配置后生效') } catch (e) { notify(`导入失败：${e instanceof Error ? e.message : e}`, true) } finally { input.value = '' } }; reader.readAsText(file) }
async function refresh(initial = false) {
  if (!desktop) return
  try {
    const result = await call<Snapshot>('snapshot')
    if (!initialized.value) { config.value = structuredClone(result.config); saved.value = structuredClone(result.config); initialized.value = true }
    running.value = result.running; logs.value = result.logs; traffic.value = result.traffic; sshRunning.value = result.ssh_running ?? false; sshLocalPort.value = result.ssh_local_port ?? null; connected.value = true
    if (result.message) notify(result.message, true)
  } catch (e) { if (connected.value || initial) notify(String(e), true); connected.value = false }
}
async function action(fn: () => Promise<void>) {
  busy.value = true
  try { await fn() } catch (e) { notify(String(e), true) } finally { busy.value = false }
}
async function persistConfig(payload: Config) { await action(async () => { await call('save_config', { config: payload }); saved.value = payload; notify('配置已保存，访问策略立即生效'); await refresh() }) }
async function save() {
  const payload: Config = JSON.parse(JSON.stringify(config.value))
  if (payload.access_mode !== saved.value.access_mode) { pendingSave.value = payload; confirmAction.value = 'save-mode'; return }
  await persistConfig(payload)
}
async function toggle() { await action(async () => { await call('set_running', { running: !running.value }); await refresh(); notify(running.value ? '代理已启动' : '代理已停止') }) }
function discard() { config.value = JSON.parse(JSON.stringify(saved.value)) }
function addRules() {
  const items = draft.value.split(/[\s,;，；]+/).map(x => x.trim().toLowerCase().replace(/\.$/, '')).filter(Boolean)
  if (!items.length) return
  const invalid = items.find(item => !/^((\*\.)?([a-z0-9-]+\.)+[a-z]{2,}|\.([a-z0-9-]+\.)+[a-z]{2,}|(\d{1,3}\.){3}\d{1,3}|localhost)$/i.test(item))
  if (invalid) { notify(`规则格式不正确：${invalid}`, true); return }
  let added = 0
  for (const item of items) { if (!activeRules.value.includes(item)) { activeRules.value.push(item); timestamps.value[item] = Math.floor(Date.now() / 1000); added++ } }
  draft.value = ''; notify('规则已添加到草稿，点击“保存配置”后生效')
}
function removeRule(rule: string) { undoRule.value = { rules: [rule], mode: config.value.access_mode }; config.value[config.value.access_mode] = activeRules.value.filter(r => r !== rule); delete timestamps.value[rule]; notify(`已删除 ${rule}，可撤销`) }
function undoLastRule() { if (!undoRule.value) return; const { rules, mode } = undoRule.value; if (mode === config.value.access_mode) config.value[mode] = [...new Set([...rules, ...config.value[mode]])]; undoRule.value = null; notify('已撤销上次规则操作') }
function mainDomain(host: string) {
  if (/^\d+(\.\d+){3}$/.test(host) || host === 'localhost') return host
  const parts = host.split('.').filter(Boolean)
  if (parts.length <= 2) return host
  const compoundSuffixes = new Set(['co.uk', 'org.uk', 'ac.uk', 'gov.uk', 'com.cn', 'net.cn', 'org.cn', 'com.hk', 'co.jp', 'com.au', 'co.nz', 'co.kr', 'co.in', 'com.br'])
  const suffix = parts.slice(-2).join('.')
  return parts.slice(-(compoundSuffixes.has(suffix) ? 3 : 2)).join('.')
}
function showTargetMenu(event: MouseEvent, target: string) {
  const host = target.trim().toLowerCase().replace(/^https?:\/\//, '').split(/[/?#]/)[0].replace(/\.$/, '')
  const parts = host.split('.').filter(Boolean)
  const candidates = [host]
  if (parts.length > 2 && !/^\d+(\.\d+){3}$/.test(host)) {
    for (let i = 1; i < parts.length - 1; i++) candidates.push(`*.${parts.slice(i).join('.')}`)
    candidates.push(`.${mainDomain(host)}`)
  }
  const available = [...new Set(candidates)].filter(rule => !activeRules.value.includes(rule))
  selectedTargets.value = available.slice(0, 1)
  contextMenu.value = { target: host, x: event.clientX, y: event.clientY, candidates: available }
}
function addTargetRules() { if (!contextMenu.value) return; const rules = selectedTargets.value.filter(rule => contextMenu.value?.candidates.includes(rule)); const now = Math.floor(Date.now() / 1000); for (const rule of rules) { if (!activeRules.value.includes(rule)) { activeRules.value.push(rule); timestamps.value[rule] = now } } notify(rules.length ? `${rules.length} 条规则已加入${modeName.value}草稿，请保存配置后生效` : '请选择至少一条规则'); if (rules.length) contextMenu.value = null }
async function clearLogs() { await action(async () => { await call('clear_logs'); logs.value = []; notify('访问日志已清空') }) }
watch([logs, autoScrollLogs], async () => { if (!autoScrollLogs.value) return; await nextTick(); if (logPanel.value) logPanel.value.scrollTop = logPanel.value.scrollHeight }, { deep: true })
async function copyRules() { try { await navigator.clipboard.writeText(activeRules.value.join('\n')); notify('规则已复制') } catch (e) { notify(`复制失败：${e}`, true) } }
async function loadPorts() { portBusy.value = true; try { ports.value = await call<PortRow[]>('list_ports'); portLoaded.value = true } catch (e) { notify(String(e), true) } finally { portBusy.value = false } }
function navigate(id: string) { tab.value = id; if (id === 'ports' && desktop && !portLoaded.value) void loadPorts() }
function addSshProfile() { const id = `ssh-${Date.now()}`; config.value.ssh_profiles.push({ id, name: `SSH 链路 ${config.value.ssh_profiles.length + 1}`, hops: [{ host: '', port: 22, username: '', auth: 'agent', keychain_id: null }], enabled: false }); config.value.active_ssh_profile = id }
function removeSshProfile(id: string) { config.value.ssh_profiles = config.value.ssh_profiles.filter(profile => profile.id !== id); if (config.value.active_ssh_profile === id) config.value.active_ssh_profile = config.value.ssh_profiles[0]?.id ?? null }
function addSshHop(profile: SshProfile) { profile.hops.push({ host: '', port: 22, username: '', auth: 'agent', keychain_id: null }) }
function removeSshHop(profile: SshProfile, index: number) { if (profile.hops.length > 1) profile.hops.splice(index, 1) }
function moveSshHop(profile: SshProfile, from: number, to: number) { if (from === to || to < 0 || to >= profile.hops.length) return; const [hop] = profile.hops.splice(from, 1); profile.hops.splice(to, 0, hop) }
function beginHopDrag(profile: SshProfile, index: number) { draggedHop.value = { profile, index } }
function dropHop(profile: SshProfile, index: number) { if (draggedHop.value?.profile === profile) moveSshHop(profile, draggedHop.value.index, index); draggedHop.value = null }
async function syncIcloud() { await action(async () => { const path = await call<string>('icloud_sync'); config.value.icloud_sync_enabled = true; notify(`配置已同步到 iCloud：${path}`) }) }
async function mergeIcloud() { await action(async () => { const remote = await call<Config | null>('icloud_read'); if (!remote) { notify('iCloud 中暂无配置'); return } config.value = { ...config.value, ...remote, whitelist: [...new Set([...config.value.whitelist, ...remote.whitelist])], blacklist: [...new Set([...config.value.blacklist, ...remote.blacklist])], ssh_profiles: [...config.value.ssh_profiles, ...remote.ssh_profiles.filter(r => !config.value.ssh_profiles.some(l => l.id === r.id))] }; notify('iCloud 配置已合并到草稿，请检查后保存') }) }
async function saveSshSecret(hop: { keychain_id?: string | null }, key: string) { const secret = sshSecrets.value[key]; if (!secret || !hop.keychain_id) { notify('请先填写钥匙串标识和凭据', true); return } await action(async () => { await call('keychain_set', { account: hop.keychain_id, secret }); sshSecrets.value[key] = ''; notify('SSH 凭据已保存到 macOS 钥匙串') }) }
async function deleteSshSecret(hop: { keychain_id?: string | null }) { if (!hop.keychain_id) return; await action(async () => { await call('keychain_delete', { account: hop.keychain_id }); notify('SSH 凭据已从 macOS 钥匙串删除') }) }
async function checkIcloud() { if (desktop) { try { icloudAvailable.value = await call<boolean>('icloud_status') } catch { icloudAvailable.value = false } } }
function closeConfirm() { confirmAction.value = null; pendingSave.value = null }
async function confirm() {
  const selected = confirmAction.value
  if (selected === 'clear') { const previous = [...activeRules.value]; undoRule.value = { rules: previous, mode: config.value.access_mode }; config.value[config.value.access_mode] = []; config.value[config.value.access_mode === 'whitelist' ? 'whitelist_added_at' : 'blacklist_added_at'] = {}; confirmAction.value = null; notify(`已清空${modeName.value}草稿，可点击撤销`) }
  else if (selected === 'save-mode' && pendingSave.value) { const payload = pendingSave.value; closeConfirm(); await persistConfig(payload) }
  else if (selected && selected !== 'save-mode') { await action(async () => { await call('terminate_process', { pid: selected.pid, started: selected.started }); confirmAction.value = null; notify(`已向 PID ${selected.pid} 发送终止信号`); await loadPorts() }) }
}
let timer: ReturnType<typeof setTimeout> | undefined
let disposed = false
async function poll() { await refresh(); if (!disposed) timer = setTimeout(poll, 1500) }
onMounted(async () => { await refresh(true); await checkIcloud(); if (!disposed) timer = setTimeout(poll, 1500) })
onUnmounted(() => { disposed = true; clearTimeout(timer) })
</script>

<template>
  <div class="app-shell" @click="contextMenu = null">
    <aside class="sidebar">
      <div class="brand"><span class="brand-mark"><Globe2 :size="24" /></span><div>DomainEgress<small>本地网络访问控制</small></div></div>
      <div class="nav-label">工作空间</div>
      <nav aria-label="主导航"><button v-for="item in tabs" :key="item.id" :class="{ active: tab === item.id }" @click="navigate(item.id)"><component :is="item.icon" :size="18" /><span>{{ item.title }}</span><ChevronRight v-if="tab === item.id" :size="15" /></button></nav>
      <div class="sidebar-bottom"><div class="local-badge"><span class="dot" :class="{ live: running }"></span>{{ running ? '代理正在运行' : '代理已停止' }}</div><p>本地网络安全代理</p><span class="version">DESKTOP / 0.1.0</span></div>
    </aside>
    <main>
      <header><div class="breadcrumb">工作空间 <ChevronRight :size="13" /> <span>{{ tabs.find(t => t.id === tab)?.title }}</span></div><div class="header-right"><span class="desktop-label">{{ desktop ? '本机桌面' : '界面预览' }}</span><span class="dot" :class="{ live: connected && desktop }"></span>{{ desktop ? (connected ? '核心已连接' : '连接中断') : '未连接核心' }}</div></header>
      <div class="content">
        <div v-if="!desktop" class="preview-banner"><CircleHelp :size="17" /> 当前为浏览器界面预览。代理操作、配置保存与端口查询请使用桌面应用。</div>
        <div class="page-heading"><div><div class="eyebrow">{{ tab === 'overview' ? 'NETWORK OVERVIEW' : tab === 'rules' ? 'ACCESS POLICY' : tab === 'logs' ? 'REQUEST LOGS' : tab === 'ports' ? 'SYSTEM NETWORK' : 'PREFERENCES' }}</div><h1>{{ tabs.find(t => t.id === tab)?.title }}</h1><p>{{ tab === 'overview' ? '让每一次网络访问，都在掌控之中。' : tab === 'rules' ? '定义允许或拒绝访问的域名与 IP 地址。' : tab === 'logs' ? '查看本次运行的访问决策与请求信息。' : tab === 'ports' ? '查看本机 TCP 监听端口及所属进程。' : '管理代理监听地址、启动行为与日志策略。' }}</p></div><button v-if="tab === 'rules' || tab === 'settings'" class="primary" :disabled="busy || !desktop || !initialized || !connected || !dirty" @click="save"><Check :size="16" />保存配置<span v-if="dirty" class="unsaved"></span></button><span v-else-if="tab === 'overview'" class="pill">HTTP / HTTPS / SOCKS5</span></div>
        <div v-if="notice" class="notice" :class="{ error }" role="status"><span>{{ notice }}</span><button v-if="undoRule" class="undo" @click="undoLastRule">撤销</button><button aria-label="关闭提示" @click="notice = ''"><X :size="16" /></button></div>
        <div v-if="dirty" class="draft-banner">有未保存的修改，保存后生效。<button @click="discard">撤销修改</button></div>

        <template v-if="tab === 'overview'">
          <section class="status-card"><div class="status-icon" :class="{ stopped: !running }"><ShieldCheck :size="32" /></div><div class="status-text"><span class="section-label">代理服务</span><h2>{{ running ? '连接已就绪' : '准备好安全连接' }}<span class="pill" :class="{ green: running }">{{ running ? '运行中' : '已停止' }}</span></h2><p>{{ running ? '正在根据访问策略处理本机代理请求' : '启动代理，为网络访问应用你的准出策略' }}</p><div class="service-endpoints"><span>HTTP/HTTPS {{ saved.http_host }}:{{ saved.http_port }}</span><span>SOCKS5 {{ saved.socks_host }}:{{ saved.socks_port }}</span></div></div><button :class="running ? 'secondary' : 'primary'" :disabled="busy || !desktop || !connected" @click="toggle"><Square v-if="running" :size="15" /><Play v-else :size="15" />{{ running ? '停止代理' : '启动代理' }}</button></section>
          <div class="metrics"><section class="metric"><span>当前访问策略<ShieldCheck :size="17" /></span><strong>{{ saved.access_mode === 'whitelist' ? '白名单' : '黑名单' }}<small>模式</small></strong><p>{{ saved[saved.access_mode].length }} 条已保存规则<button @click="navigate('rules')">管理规则 <ArrowUpRight :size="14" /></button></p></section><section class="metric"><span>{{ trendRangeLabel }}放行<ArrowUpRight :size="17" /></span><strong>{{ admitted.toLocaleString() }}<small>次</small></strong><p>按当前趋势范围统计</p></section><section class="metric"><span>已记录的拦截<ShieldCheck :size="17" /></span><strong>{{ blocked.toLocaleString() }}<small>次</small></strong><p>当前内存日志中的拦截记录</p></section></div>
          <section class="panel"><div class="section-heading"><div><h3>访问趋势</h3><p>{{ trendRangeLabel }} · 策略放行次数</p></div><div class="toolbar"><select v-model.number="trendRange" aria-label="趋势范围"><option :value="10">10 分钟</option><option :value="30">30 分钟</option><option :value="60">1 小时</option><option :value="180">3 小时</option><option :value="360">6 小时</option><option :value="720">12 小时</option><option :value="1440">24 小时</option></select><select v-model.number="trendInterval" aria-label="趋势统计间隔"><option :value="30">30 秒</option><option :value="60">1 分钟</option><option :value="300">5 分钟</option><option :value="600">10 分钟</option><option :value="1800">30 分钟</option></select><span class="legend"><span class="dot live"></span>已放行请求</span></div></div><div class="chart-scroll"><svg class="line-chart" :width="Math.max(720, buckets.length * 54)" height="220" role="img" :aria-label="trendRangeLabel + '放行 ' + admitted + ' 次'"><polyline :points="buckets.map((b, i) => (i * 54 + 30) + ',' + (190 - b.count / maximum * 160)).join(' ')" fill="none" stroke="var(--accent)" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" /><circle v-for="(bucket, i) in buckets" :key="bucket.label + i" :cx="i * 54 + 30" :cy="190 - bucket.count / maximum * 160" r="4" fill="var(--accent)" /><text v-for="(bucket, i) in buckets" :key="'label-' + bucket.label + i" :x="i * 54 + 30" y="214" text-anchor="middle">{{ bucket.label }}</text></svg></div></section>
          <div class="endpoints"><section class="endpoint"><span class="endpoint-icon"><Globe2 :size="22" /></span><div><h3>HTTP / HTTPS</h3><code>{{ saved.http_host }}:{{ saved.http_port }}</code></div><span class="pill">CONNECT</span></section><section class="endpoint"><span class="endpoint-icon"><Network :size="22" /></span><div><h3>SOCKS5</h3><code>{{ saved.socks_host }}:{{ saved.socks_port }}</code></div><span class="pill">TCP</span></section></div>
          <div class="footnote"><CircleHelp :size="15" />请在浏览器或其他应用中手动配置上述代理地址。</div>
        </template>

        <template v-if="tab === 'rules'">
          <section class="panel"><div class="section-heading"><div><h3>准出策略</h3><p>白名单仅允许匹配的地址；黑名单拒绝匹配的地址。</p></div><div class="segmented"><button :class="{ selected: config.access_mode === 'whitelist' }" @click="config.access_mode = 'whitelist'">白名单</button><button :class="{ selected: config.access_mode === 'blacklist' }" @click="config.access_mode = 'blacklist'">黑名单</button></div></div><div class="rule-hint"><ShieldCheck :size="18" />{{ config.access_mode === 'whitelist' ? '白名单为空时，拒绝全部访问。' : '黑名单为空时，允许全部访问。' }} 精确域名不会自动匹配子域名。</div><label class="field-label" for="rules-input">添加域名或 IP</label><textarea id="rules-input" v-model="draft" rows="3" placeholder="example.com&#10;*.example.com&#10;每行一条，或使用逗号分隔"></textarea><div class="toolbar"><span class="muted">*.example.com 只匹配一级子域名</span><button class="primary" :disabled="!draft.trim()" @click="addRules"><Plus :size="16" />添加规则</button></div></section>
          <section class="panel"><div class="section-heading"><h3>{{ modeName }} <span class="count">{{ orderedRules.length }} / {{ activeRules.length }}</span></h3><div class="toolbar"><div class="search rule-search"><Search :size="16" /><input v-model="ruleSearch" aria-label="搜索规则" placeholder="模糊搜索规则"></div><select v-model="ruleSort" aria-label="规则排序"><option value="name">按名称排序</option><option value="time">按添加时间</option></select><button aria-label="复制规则" title="复制规则" @click="copyRules"><Copy :size="17" /></button><button class="danger-text" :disabled="!activeRules.length" @click="confirmAction = 'clear'">清空</button></div></div><div class="table-wrap"><table><thead><tr><th>域名 / IP 地址</th><th>添加时间</th><th class="right">操作</th></tr></thead><tbody><tr v-for="rule in orderedRules" :key="rule"><td class="mono"><Globe2 :size="15" class="inline-icon" />{{ rule }}</td><td class="muted">{{ ruleDate(timestamps[rule] || 0) }}</td><td class="right"><button :aria-label="`删除 ${rule}`" class="danger-text" @click="removeRule(rule)"><Trash2 :size="16" /></button></td></tr></tbody></table><div v-if="!orderedRules.length" class="empty">{{ activeRules.length ? '没有匹配的规则。' : `当前${modeName}为空，请添加规则。` }}</div></div></section>
        </template>

        <template v-if="tab === 'logs'">
          <section class="panel"><div class="section-heading"><div><div class="search"><Search :size="17" /><input v-model="search" aria-label="搜索日志" placeholder="搜索来源、方法、目标或参数"></div><div class="toolbar log-controls"><select v-model="logLevel" aria-label="日志级别"><option value="all">全部级别</option><option value="error">ERROR</option><option value="warn">WARN</option><option value="info">INFO</option><option value="debug">DEBUG</option></select><select v-model="logOutcome" aria-label="结果"><option value="all">全部结果</option><option value="放行">放行</option><option value="拦截">拦截</option></select><select v-model.number="trendRange" aria-label="拦截统计时间"><option :value="10">最近 10 分钟</option><option :value="30">最近 30 分钟</option><option :value="60">最近 1 小时</option><option :value="180">最近 3 小时</option><option :value="360">最近 6 小时</option><option :value="720">最近 12 小时</option><option :value="1440">最近 24 小时</option></select><button :class="{ selected: autoScrollLogs }" @click="autoScrollLogs = !autoScrollLogs"><ArrowUpRight :size="15" />实时滚动日志</button><button class="danger-text" :disabled="!logs.length || !desktop" @click="clearLogs"><Trash2 :size="15" />删除日志</button></div></div></div><div ref="logPanel" class="table-wrap"><table><thead><tr><th>时间 / 来源</th><th>级别</th><th>方法 / 目标</th><th>参数</th><th>结果</th></tr></thead><tbody><tr v-for="(log, index) in filteredLogs" :key="index"><td>{{ date(log.timestamp) }}<small class="cell-sub mono">{{ log.source }}</small></td><td><span class="pill">{{ log.level.toUpperCase() }}</span></td><td><strong>{{ log.method }}</strong><small class="cell-sub mono" @contextmenu.prevent.stop="showTargetMenu($event, log.target)">{{ log.target }}</small></td><td class="params">{{ log.params || '—' }}</td><td><span class="pill" :class="log.outcome === '拦截' ? 'red' : 'green'">{{ log.outcome }}</span></td></tr></tbody></table><div v-if="!filteredLogs.length" class="empty"><ListFilter :size="30" /><h3>{{ logs.length ? '没有符合条件的日志' : '暂无访问日志' }}</h3><p>代理收到请求后，符合日志级别的记录将显示在这里。</p></div></div><p class="footnote">显示 {{ filteredLogs.length }} 条 · 每 1.5 秒刷新 · 最多 2,000 条内存日志，退出后清空。</p></section>
          <section class="panel"><div class="section-heading"><div><h3>拦截域名统计 <span class="count">{{ blockedDomains.length }}</span></h3><p>按目标域名与请求来源去重 · {{ trendRangeLabel }}</p></div></div><div class="table-wrap"><table><thead><tr><th>拦截域名</th><th>请求来源</th><th>拦截次数</th><th>最近拦截时间</th></tr></thead><tbody><tr v-for="(log, index) in blockedDomains" :key="`${log.target}-${log.source}-${index}`"><td class="mono">{{ log.target }}</td><td class="mono">{{ log.source }}</td><td>{{ log.count }}</td><td class="muted">{{ date(log.timestamp) }}</td></tr></tbody></table><div v-if="!blockedDomains.length" class="empty">当前时间范围内没有拦截域名。</div></div></section>
        </template>

        <template v-if="tab === 'ports'">
          <section class="panel"><div class="section-heading"><div class="search"><Search :size="17" /><input v-model="portSearch" aria-label="搜索端口" placeholder="搜索端口、进程或 PID"></div><button :disabled="portBusy || !desktop" @click="loadPorts"><RefreshCw :size="16" :class="{ spin: portBusy }" />{{ portBusy ? '查询中…' : '刷新' }}</button></div><div class="table-wrap"><table><thead><tr><th>监听地址</th><th>进程 / PID</th><th>启动时间 / 已运行</th><th class="right">操作</th></tr></thead><tbody><tr v-for="p in filteredPorts" :key="`${p.pid}-${p.port}`"><td class="mono">{{ p.port }}</td><td>{{ p.name }}<small class="cell-sub mono">{{ p.pid }}</small></td><td>{{ p.started }}<small class="cell-sub">{{ p.elapsed }}</small></td><td class="right"><button class="danger-text" @click="confirmAction = p">结束进程</button></td></tr></tbody></table><div v-if="!filteredPorts.length" class="empty">{{ portBusy ? '正在读取系统监听端口…' : '没有可显示的监听端口' }}</div></div><p class="footnote">数据来自 macOS lsof，只显示当前用户可见的 TCP 监听进程。</p></section>
        </template>

        <template v-if="tab === 'settings'">
          <section class="panel appearance-panel">
            <div class="section-heading"><div><h3>外观与主题</h3><p>即时生效并自动保存到本机，无需点击“保存配置”。</p></div></div>
            <div class="appearance-modes" role="group" aria-label="外观模式">
              <button v-for="mode in (['system', 'light', 'dark'] as const)" :key="mode" :aria-pressed="appearance === mode" :class="{ selected: appearance === mode }" @click="setAppearance(mode)">{{ { system: '跟随系统', light: '浅色模式', dark: '暗黑模式' }[mode] }}</button>
            </div>
            <div class="theme-grid" role="group" aria-label="程序员颜色主题">
              <button v-for="item in themes" :key="item.id" class="theme-card" :class="{ selected: theme === item.id }" :aria-pressed="theme === item.id" :aria-label="`使用 ${item.name} 主题`" @click="setTheme(item.id)">
                <span class="theme-preview" :style="{ background: item.bg, color: item.dark }"><span>&lt;/&gt;</span><i :style="{ background: item.dark }"></i><i :style="{ background: item.light }"></i></span>
                <span class="theme-name">{{ item.name }}<Check v-if="theme === item.id" :size="15" /></span><small>{{ item.description }}</small>
              </button>
            </div>
            <p v-if="appearanceError" class="appearance-error" role="status">{{ appearanceError }}</p>
          </section>
          <section class="panel"><div class="section-heading"><div><h3>代理监听</h3><p>{{ running ? '代理运行中，停止后可修改监听地址与端口。' : '默认仅监听本机回环地址。' }}</p></div><Network :size="21" class="muted" /></div><div class="form-grid"><label>HTTP / HTTPS 地址<input v-model="config.http_host" :disabled="running" placeholder="127.0.0.1"></label><label>端口<input v-model.number="config.http_port" :disabled="running" type="number" min="1" max="65535"></label><label>SOCKS5 地址<input v-model="config.socks_host" :disabled="running" placeholder="127.0.0.1"></label><label>端口<input v-model.number="config.socks_port" :disabled="running" type="number" min="1" max="65535"></label></div></section>
          <section class="panel"><h3>启动与日志</h3><label class="setting-row"><span><strong>自动启动代理</strong><small>打开应用时自动启动代理服务</small></span><input v-model="config.auto_start" type="checkbox" role="switch"></label><div class="form-grid"><label>日志级别<select v-model="config.log_level"><option value="error">ERROR · 错误</option><option value="warn">WARN · 警告</option><option value="info">INFO · 信息</option><option value="debug">DEBUG · 调试</option></select></label><label>日志保留天数<input v-model.number="config.log_retention_days" type="number" min="1" max="3650"></label><label>趋势保留天数<input v-model.number="config.trend_retention_days" type="number" min="1" max="21"></label></div><p class="footnote">内存日志在写入时按保留天数清理；最多保留 2,000 条，应用退出后清空。</p></section>
          <section class="panel ssh-panel"><div class="section-heading"><div><h3>SSH 代理链路</h3><p>放行后的流量可通过当前启用的多跳 SSH 动态 SOCKS5 出口访问目标。</p></div><button @click="addSshProfile"><Plus :size="16" />新建链路</button></div><div v-if="config.ssh_profiles.length" class="ssh-profiles"><article v-for="profile in config.ssh_profiles" :key="profile.id" class="ssh-profile"><div class="ssh-profile-head"><label class="profile-active"><input v-model="config.active_ssh_profile" type="radio" name="active-ssh" :value="profile.id">启用</label><input v-model="profile.name" class="profile-name" aria-label="SSH 链路名称"><span class="pill" :class="{ green: sshRunning && config.active_ssh_profile === profile.id }">{{ sshRunning && config.active_ssh_profile === profile.id ? `运行中 :${sshLocalPort}` : `${profile.hops.length} 跳` }}</span><button class="danger-text" @click="removeSshProfile(profile.id)">删除</button></div><div v-for="(hop, index) in profile.hops" :key="index" class="ssh-hop" draggable="true" @dragstart="beginHopDrag(profile, index)" @dragover.prevent @drop="dropHop(profile, index)"><span class="hop-index" title="拖动调整顺序">⠿ {{ index + 1 }}</span><input v-model="hop.host" placeholder="服务器地址" aria-label="SSH 服务器地址"><input v-model.number="hop.port" type="number" min="1" max="65535" placeholder="端口" aria-label="SSH 端口"><input v-model="hop.username" placeholder="用户名" aria-label="SSH 用户名"><select v-model="hop.auth" aria-label="认证方式"><option value="agent">ssh-agent</option><option value="keychain">钥匙串私钥</option><option value="password">密码（暂未启用）</option></select><button v-if="profile.hops.length > 1" class="danger-text" :aria-label="`删除第 ${index + 1} 跳`" @click="removeSshHop(profile, index)"><Trash2 :size="15" /></button></div><div class="toolbar"><button @click="addSshHop(profile)"><Plus :size="14" />添加下一跳</button><span class="muted">多跳按从前到后的顺序连接</span></div></article></div><div v-else class="empty ssh-empty">尚未配置 SSH 链路。点击“新建链路”开始配置。</div><p class="footnote"><CircleHelp :size="15" />SSH 私钥、密码和私钥口令不会同步到 iCloud；密码认证功能将在 Keychain 接入后启用。</p></section>
          <section class="panel"><div class="section-heading"><div><h3>iCloud 配置同步</h3><p>同步规则、普通代理设置和 SSH 链路元数据；私钥、密码和私钥口令不会同步。</p></div><span class="pill" :class="{ green: icloudAvailable }">{{ icloudAvailable ? 'iCloud 目录可用' : '未检测到 iCloud 目录' }}</span></div><div class="toolbar"><button class="primary" :disabled="busy || !desktop || !icloudAvailable" @click="syncIcloud">推送当前配置</button><button :disabled="busy || !desktop || !icloudAvailable" @click="mergeIcloud">拉取并合并</button><span class="muted">规则自动去重合并，其他设置以当前草稿为准</span></div></section>
          <section class="panel about"><span class="brand-mark"><Globe2 :size="24" /></span><div><h3>DomainEgress</h3><p>版本 0.1.0 · 本地网络安全代理</p><p>关闭窗口后驻留托盘，使用托盘菜单重新打开或退出。</p></div></section>
        </template>
      </div>
    </main>
    <div v-if="contextMenu" class="context-menu" :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }" @click.stop><strong>添加到{{ modeName }}</strong><small class="context-target">{{ contextMenu.target }}</small><label v-for="rule in contextMenu.candidates" :key="rule" class="candidate"><input v-model="selectedTargets" type="checkbox" :value="rule"><span>{{ rule }}</span></label><p v-if="!contextMenu.candidates.length" class="context-empty">候选规则均已存在</p><button class="primary" :disabled="!selectedTargets.length || !contextMenu.candidates.length" @click="addTargetRules">添加所选规则</button></div><div v-if="confirmAction" class="modal-backdrop" @click.self="closeConfirm"><section class="modal" role="dialog" aria-modal="true" aria-labelledby="confirm-title"><h2 id="confirm-title">{{ confirmAction === 'clear' ? `清空${modeName}？` : confirmAction === 'save-mode' ? `切换为${modeName}模式？` : '结束进程？' }}</h2><p>{{ confirmAction === 'clear' ? '将清空当前列表草稿，保存后才会生效。' : confirmAction === 'save-mode' ? `将切换为${modeName}模式。保存后，新的准出策略将立即作用于后续连接。` : `将向 ${confirmAction.name}（PID ${confirmAction.pid}）发送 SIGTERM，可能中断该应用的连接。` }}</p><div class="toolbar"><button :disabled="busy" @click="closeConfirm">取消</button><button class="danger" :disabled="busy" @click="confirm">{{ busy ? '处理中…' : '确认' }}</button></div></section></div>
  </div>
</template>

<style scoped>
:global(body) { overflow-x: hidden; }
.context-menu { position: fixed; z-index: 30; background: var(--surface); border: 1px solid var(--border); border-radius: 7px; padding: 5px; box-shadow: 0 8px 24px var(--shadow); }
.context-menu button { border: 0; width: 100%; }
.context-menu { min-width: 280px; max-width: min(360px, calc(100vw - 24px)); padding: 12px; }
.context-menu strong, .context-target { display:block; }
.context-target { color:var(--muted); font:10px "SFMono-Regular",Consolas,monospace; margin:6px 0 10px; overflow-wrap:anywhere; }
.candidate { display:flex; align-items:center; gap:8px; padding:8px 4px; font:11px "SFMono-Regular",Consolas,monospace; cursor:pointer; }
.candidate input { accent-color:var(--accent); }
.context-menu .primary { margin-top:8px; }
.context-empty { font-size:11px; padding:4px 0; }
.ssh-profile { border:1px solid var(--border); border-radius:9px; padding:16px; margin-bottom:12px; }
.ssh-profile-head { display:flex; align-items:center; gap:10px; margin-bottom:13px; }
.profile-active { display:flex; align-items:center; gap:6px; color:var(--muted); font-size:11px; }
.profile-name { flex:1; font-weight:600; min-width:120px; }
.ssh-hop { display:grid; grid-template-columns:28px 2fr 90px 1.5fr 1.2fr 36px; gap:8px; align-items:center; margin-bottom:9px; }
.hop-index { display:flex; align-items:center; justify-content:center; width:24px; height:24px; background:var(--raised); border-radius:50%; color:var(--muted); font-size:11px; cursor:grab; user-select:none; }
.ssh-hop:active .hop-index { cursor:grabbing; }
.ssh-profile .toolbar { margin-top:10px; }
.ssh-empty { padding:28px 15px; }
@media (max-width:700px) { .ssh-profile-head { flex-wrap:wrap; } .profile-name { order:2; flex-basis:calc(100% - 80px); } .ssh-hop { grid-template-columns:28px 1fr 80px; } .ssh-hop input:nth-of-type(2), .ssh-hop select { grid-column:2 / 4; } .ssh-hop button { grid-column:3; grid-row:1; } }
.chart-scroll { width: 100%; overflow-x: auto; overflow-y: hidden; }
.line-chart { display: block; min-width: 720px; }
.line-chart text { fill: var(--muted); font-size: 10px; }
.service-endpoints { display:flex; gap:14px; margin-top:10px; color:var(--muted); font:10px "SFMono-Regular",Consolas,monospace; }
.notice .undo { color:var(--accent); font-weight:600; padding:4px 8px; background:none; border:0; }
@media (max-width:700px) { .service-endpoints { flex-direction:column; gap:4px; } .status-text { min-width:0; } .service-endpoints span { overflow:hidden; text-overflow:ellipsis; } }
</style>
