import { useCloudResources } from './useCloudResources';
import { useNetworkTools } from './useNetworkTools';
import { useLocalDns } from './useLocalDns';
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { Activity, AlertTriangle, ArrowUpRight, Check, ChevronLeft, ChevronRight, CircleHelp, CircleStop, Cloud, Copy, Download, FileKey2, Globe2, ListFilter, Network, Play, Plus, Radio, RefreshCw, Search, Server, Settings2, ShieldCheck, Square, Trash2, Upload, Wifi, X, } from "lucide-vue-next";
import { appearance, theme, themes, appearanceError, setAppearance, setTheme, } from "../theme";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { call, defaults, desktop, type CloudAccount, type CloudRegion, type CloudSecurityGroup, type CloudSecurityRule, type Config, type CreateManagedRuleInput, type EgressAddress, type HostGroup, type HostMapping, type LogEntry, type ManagedRuleConfig, type ManagedRuleSyncResult, type NetworkInterface, type NetworkProbeRequest, type NetworkProbeResult, type NetworkTool, type PortRow, type PublicIpProbe, type SaveCloudAccountInput, type Snapshot, type SshForwardRule, type SshProfile, type UpdateInfo, } from "../api";
export function useWorkspace() {
    const tabs = [
        { id: "overview", title: "代理概览", icon: Activity },
        { id: "rules", title: "访问控制", icon: ShieldCheck },
        { id: "logs", title: "访问日志", icon: ListFilter },
        { id: "ports", title: "监听端口", icon: Network },
        { id: "ssh-forward", title: "SSH 端口转发", icon: Network },
        { id: "static-http", title: "静态 HTTP 服务", icon: Server },
        { id: "network-tools", title: "网络探测", icon: Radio },
        { id: "local-dns", title: "本地 DNS", icon: Server },
        { id: "settings", title: "应用设置", icon: Settings2 },
    ];
    const tab = ref("overview"), config = ref<Config>(structuredClone(defaults)), saved = ref<Config>(structuredClone(defaults));
    const pageTitle = computed(() => tab.value === "cloud"
        ? "云资源"
        : tabs.find((item) => item.id === tab.value)?.title || "");
    const pageEyebrow = computed(() => ({
        overview: "NETWORK OVERVIEW",
        rules: "ACCESS POLICY",
        logs: "REQUEST LOGS",
        ports: "SYSTEM NETWORK",
        "ssh-forward": "SSH PORT FORWARDING",
        "static-http": "LOCAL STATIC HTTP",
        "network-tools": "NETWORK DIAGNOSTICS",
        "local-dns": "LOCAL HOSTS",
        cloud: "CLOUD RESOURCES",
        settings: "PREFERENCES",
    })[tab.value] || "WORKSPACE");
    const pageDescription = computed(() => ({
        overview: "让每一次网络访问，都在掌控之中。",
        rules: "定义允许或拒绝访问的域名与 IP 地址。",
        logs: "查看本次运行的访问决策与请求信息。",
        ports: "查看本机 TCP 监听端口及所属进程。",
        "ssh-forward": "将远程服务器端口安全映射到本机访问。",
        "network-tools": "从本机直接验证目标的可达性、协议和证书。",
        "local-dns": "维护当前 Mac 的应用专属 /etc/hosts 映射。",
        cloud: "按云账号管理远程资源与网络访问能力。",
        "static-http": "临时共享本地静态资源，按端口管理目录并查看客户端访问日志。",
        settings: "管理代理监听地址、启动行为与日志策略。",
    })[tab.value] || "");
    const running = ref(false), logs = ref<LogEntry[]>([]), traffic = ref<number[]>([]), busy = ref(false), connected = ref(!desktop), initialized = ref(false), sshRunning = ref(false), sshLocalPort = ref<number | null>(null), icloudAvailable = ref(false);
    const interfaces = ref<NetworkInterface[]>([]);
    const hostname = ref("本机");
    const emptyEgressProbe = (): PublicIpProbe["system"] => ({
        addresses: [],
        confidence: "未探测",
        error: null,
    });
    const minimumSpinnerDuration = 420;
    const publicIp = ref<PublicIpProbe>({ system: emptyEgressProbe(), ipv6: null }), publicIpBusy = ref(false);
    const { cloudAccounts, cloudLoading, cloudAccountDialog, cloudSubtab, cloudAccountForm, cloudRegions, cloudSecurityGroups, cloudRegionLoading, cloudGroupsLoading, selectedCloudAccountId, selectedCloudRegion, selectedSecurityGroupId, cloudSecurityRules, cloudRulesLoading, managedRules, managedRuleDialog, managedRuleForm, managedRuleSourceCidr, managedRuleSourceLoading, managedRuleSourceError, cloudRegionRequest, cloudGroupRequest, cloudRuleRequest, managedSourceRequest, cloudSyncRunning, loadCloudAccounts, openCloudAccountDialog, saveCloudAccount, verifyCloudAccount, loadCloudRegions, loadCloudSecurityGroups, selectCloudSecurityGroup, loadCloudSecurityRules, loadManagedRules, refreshManagedRuleSource, openManagedRuleDialog, createManagedRule, syncManagedRule, syncAllManagedRules, deleteManagedRule, seedCloudPreview, deleteCloudAccount } = useCloudResources({ notify, action, busy });
    const sshForwardPorts = ref<Record<string, number>>({});
    const sshCommand = ref("");
    const showSshCommandParser = ref(false), parsedForwardIds = ref<string[]>([]);
    const updateInfo = ref<UpdateInfo | null>(null), updateBusy = ref(false), updateDismissed = ref(false);
    const notice = ref(""), error = ref(false), draft = ref(""), search = ref(""), logLevel = ref("all"), logOutcome = ref("all"), ruleSort = ref("name"), ruleSearch = ref(""), trendRange = ref(60), trendInterval = ref(60), autoScrollLogs = ref(true), trendStart = ref(""), trendEnd = ref(""), undoRule = ref<{
        rules: string[];
        mode: "whitelist" | "blacklist";
    } | null>(null);
    const ports = ref<PortRow[]>([]), portBusy = ref(false), portLoaded = ref(false), portSearch = ref(""), confirmAction = ref<"clear" | "save-mode" | PortRow | null>(null), pendingSave = ref<Config | null>(null), contextMenu = ref<{
        target: string;
        x: number;
        y: number;
        candidates: string[];
    } | null>(null), selectedTargets = ref<string[]>([]), sshSecrets = ref<Record<string, string>>({}), gistToken = ref(""), draggedHop = ref<{
        profile: SshProfile;
        index: number;
    } | null>(null);
    const logPanel = ref<HTMLElement | null>(null);
    const fontScaleChoices = [90, 100, 110, 125];
    const dirty = computed(() => JSON.stringify(config.value) !== JSON.stringify(saved.value));
    const showGlobalOptions = ref(false);
    const autostartEnabled = ref(false), autostartBusy = ref(false);
    const expandedRuleOptions = ref<Record<string, boolean>>({});
    const { networkTool, networkProbe, networkResult, networkBusy, networkTools, selectNetworkTool, detailText, runNetworkProbe, cancelNetworkProbe } = useNetworkTools({ notify });
    const { hostMappings, hostsLoading, hostForm, hostBatchText, hostBatchGroup, hostGroups, selectedDnsGroup, hostGroupContent, selectedDnsGroupDirty, systemHostsContent, selectedDnsView, hostGroupNameDraft, selectedHostGroup, editingHostKey, editingHostOriginal, editingHostForm, systemEditorGutter, systemEditorCode, groupEditorGutter, groupEditorCode, systemEditorLines, groupEditorLines, isHostsComment, syncHostsEditorScroll, loadHostMappings, loadHostGroups, loadSystemHosts, selectSystemHosts, selectDnsGroup, saveDnsGroup, toggleDnsGroup, createDnsGroup, deleteDnsGroup, addHostMapping, parseHostBatch, toggleHostBatchComments, addHostBatch, hostMappingKey, beginEditHostMapping, cancelEditHostMapping, saveHostMappingEdit, removeHostMapping, hostMappingGroups, selectedHostMappings, selectedDnsEntries } = useLocalDns({ notify, action });
    function optionParts(option: string) {
        const [key, ...rest] = option.split("=");
        return { key, value: rest.join("=") };
    }
    function addGlobalOption() {
        config.value.ssh_forward_options.push("NewOption=");
        showGlobalOptions.value = true;
    }
    function updateGlobalOption(index: number, key: string, value: string) {
        config.value.ssh_forward_options[index] = `${key.trim()}=${value}`;
    }
    function removeGlobalOption(index: number) {
        config.value.ssh_forward_options.splice(index, 1);
    }
    function toggleRuleOptions(id: string) {
        expandedRuleOptions.value[id] = !expandedRuleOptions.value[id];
    }
    function addRuleOption(rule: SshForwardRule) {
        rule.ssh_options.push("NewOption=");
        expandedRuleOptions.value[rule.id] = true;
    }
    function updateRuleOption(rule: SshForwardRule, index: number, key: string, value: string) {
        rule.ssh_options[index] = `${key.trim()}=${value}`;
    }
    function removeRuleOption(rule: SshForwardRule, index: number) {
        rule.ssh_options.splice(index, 1);
    }
    const modeName = computed(() => config.value.access_mode === "whitelist" ? "白名单" : "黑名单");
    const activeRules = computed(() => config.value[config.value.access_mode]);
    const timestamps = computed(() => config.value[config.value.access_mode === "whitelist"
        ? "whitelist_added_at"
        : "blacklist_added_at"]);
    const orderedRules = computed(() => [...activeRules.value]
        .filter((r) => r.includes(ruleSearch.value.trim().toLowerCase()))
        .sort((a, b) => ruleSort.value === "name"
        ? a.localeCompare(b)
        : (timestamps.value[b] || 0) - (timestamps.value[a] || 0)));
    const filteredLogs = computed(() => logs.value
        .filter((l) => (logLevel.value === "all" || logLevel.value === l.level) &&
        (logOutcome.value === "all" || logOutcome.value === l.outcome) &&
        Object.values(l)
            .join(" ")
            .toLowerCase()
            .includes(search.value.toLowerCase()))
        .reverse());
    const range = computed(() => {
        const end = Date.now() / 1000;
        return { start: end - trendRange.value * 60, end };
    });
    const blockedLogs = computed(() => logs.value.filter((l) => l.outcome === "拦截" &&
        l.timestamp >= range.value.start &&
        l.timestamp <= range.value.end));
    const blockedDomains = computed(() => {
        const grouped = new Map<string, LogEntry & {
            count: number;
        }>();
        for (const log of blockedLogs.value) {
            const key = `${log.target}\u0000${log.source}`;
            const item = grouped.get(key);
            if (item)
                item.count += 1;
            else
                grouped.set(key, { ...log, count: 1 });
        }
        return [...grouped.values()].sort((a, b) => b.count - a.count || b.timestamp - a.timestamp);
    });
    const filteredPorts = computed(() => ports.value.filter((p) => Object.values(p)
        .join(" ")
        .toLowerCase()
        .includes(portSearch.value.toLowerCase())));
    const groupedSshForwards = computed(() => {
        const groups = new Map<string, SshForwardRule[]>();
        for (const rule of config.value.ssh_forwards)
            groups.set(rule.project.trim() || "未分组", [
                ...(groups.get(rule.project.trim() || "未分组") || []),
                rule,
            ]);
        return [...groups].map(([name, rules]) => ({ name, rules }));
    });
    const expandedForwardGroup = ref("");
    function isForwardGroupExpanded(name: string, index: number) {
        return expandedForwardGroup.value
            ? expandedForwardGroup.value === name
            : index === groupedSshForwards.value.length - 1;
    }
    function toggleForwardGroup(name: string, index: number) {
        expandedForwardGroup.value = isForwardGroupExpanded(name, index)
            ? "__none__"
            : name;
    }
    const selectedForwardGroup = ref("");
    const forwardMenuCollapsed = ref(false);
    const selectedForwardRules = computed(() => groupedSshForwards.value.find((group) => group.name === selectedForwardGroup.value)?.rules || []);
    function selectForwardGroup(name: string) {
        selectedForwardGroup.value = selectedForwardGroup.value === name ? "" : name;
    }
    watch(groupedSshForwards, (groups) => {
        if (!groups.some((group) => group.name === selectedForwardGroup.value))
            selectedForwardGroup.value = groups.at(-1)?.name || "";
    }, { immediate: true });
    const localAddressSummary = computed(() => [
        ...new Set(interfaces.value
            .flatMap((item) => item.addresses)
            .filter((address) => !address.includes(":"))),
    ].join(" · ") || "未检测到");
    const egressProbes = computed(() => [
        { label: "IPv4 出口", value: publicIp.value.system },
        ...(publicIp.value.ipv6
            ? [{ label: "IPv6 出口", value: publicIp.value.ipv6 }]
            : []),
    ]);
    const buckets = computed(() => {
        const step = trendInterval.value;
        const start = Math.floor(range.value.start / step) * step;
        const end = Math.ceil(range.value.end / step) * step;
        const count = Math.min(1440, Math.max(1, Math.ceil((end - start) / step)));
        return Array.from({ length: count }, (_, i) => {
            const bucket = start + i * step;
            return {
                label: new Date(bucket * 1000).toLocaleString("zh-CN", {
                    month: "numeric",
                    day: "numeric",
                    hour: "2-digit",
                    minute: "2-digit",
                }),
                count: traffic.value.filter((t) => t >= bucket &&
                    t < bucket + step &&
                    t >= range.value.start &&
                    t <= range.value.end).length,
            };
        });
    });
    const maximum = computed(() => Math.max(1, ...buckets.value.map((b) => b.count)));
    const trendRangeLabel = computed(() => trendRange.value < 60
        ? `最近 ${trendRange.value} 分钟`
        : `最近 ${trendRange.value / 60} 小时`);
    const admitted = computed(() => buckets.value.reduce((n, b) => n + b.count, 0));
    const blocked = computed(() => logs.value.filter((l) => l.outcome === "拦截").length);
    const date = (t: number) => t
        ? new Date(t * 1000).toLocaleString("zh-CN", { hour12: false })
        : "历史规则";
    const ruleDate = (t: number) => (t ? date(t) : "内置规则");
    let noticeTimer: ReturnType<typeof setTimeout> | undefined;
    function notify(message: string, failed = false) {
        notice.value = message;
        error.value = failed;
        clearTimeout(noticeTimer);
        noticeTimer = setTimeout(() => {
            if (notice.value === message) {
                notice.value = "";
                undoRule.value = null;
            }
        }, 3000);
    }
    function adjustFontScale(direction: -1 | 1) {
        const current = fontScaleChoices.indexOf(config.value.font_scale);
        const next = Math.max(0, Math.min(fontScaleChoices.length - 1, (current < 0 ? 1 : current) + direction));
        config.value.font_scale = fontScaleChoices[next];
    }
    function resetFontScale() {
        config.value.font_scale = 100;
    }
    function handleFontScaleShortcut(event: KeyboardEvent) {
        if (!event.ctrlKey || event.altKey || event.metaKey)
            return;
        if (event.key === "+" || event.key === "=" || event.code === "NumpadAdd") {
            event.preventDefault();
            adjustFontScale(1);
        }
        else if (event.key === "-" || event.code === "NumpadSubtract") {
            event.preventDefault();
            adjustFontScale(-1);
        }
        else if (event.key === "0" || event.code === "Numpad0") {
            event.preventDefault();
            resetFontScale();
        }
    }
    async function handleCopyClick(event: MouseEvent) {
        contextMenu.value = null;
        const target = event.target as HTMLElement;
        if (target.closest("button, input, textarea, select, a"))
            return;
        const element = target.closest(".interface-address, code, .mono, .service-endpoints span, .public-ip-result strong") as HTMLElement | null;
        const value = element?.innerText?.trim();
        if (!value || !desktop)
            return;
        try {
            await navigator.clipboard.writeText(value);
            notify(`已复制：${value}`);
        }
        catch {
            notify("复制失败，请检查系统剪贴板权限", true);
        }
    }
    function locationLabel(location: EgressAddress["location"]) {
        return location
            ? [location.country, location.region, location.city]
                .filter(Boolean)
                .join(" · ")
            : "";
    }
    async function keepSpinnerVisible(startedAt: number) {
        const remaining = minimumSpinnerDuration - (performance.now() - startedAt);
        if (remaining > 0)
            await new Promise<void>((resolve) => window.setTimeout(resolve, remaining));
    }
    async function probePublicIp() {
        publicIpBusy.value = true;
        const startedAt = performance.now();
        await nextTick();
        try {
            publicIp.value = await call<PublicIpProbe>("probe_public_ip");
        }
        catch (e) {
            publicIp.value = {
                system: {
                    ...emptyEgressProbe(),
                    confidence: "探测失败",
                    error: String(e),
                },
                ipv6: null,
            };
        }
        finally {
            await keepSpinnerVisible(startedAt);
            publicIpBusy.value = false;
        }
    }
    async function checkUpdate() {
        if (!desktop)
            return;
        updateBusy.value = true;
        const startedAt = performance.now();
        await nextTick();
        try {
            updateInfo.value = await call<UpdateInfo>("check_update");
        }
        catch (e) {
            updateInfo.value = {
                current_version: "0.5.0",
                latest_version: null,
                release_url: null,
                available: false,
                error: String(e),
            };
        }
        finally {
            await keepSpinnerVisible(startedAt);
            updateBusy.value = false;
        }
    }
    async function openUpdate() {
        if (updateInfo.value?.release_url)
            await call("open_update", { url: updateInfo.value.release_url });
    }
    function localInput(t: number) {
        const d = new Date(t * 1000 - new Date().getTimezoneOffset() * 60000);
        return d.toISOString().slice(0, 16);
    }
    function setTrend(value: number) {
        trendRange.value = value;
    }
    function exportRules() {
        const payload = {
            format: "domain-egress-rules",
            version: 1,
            exported_at: new Date().toISOString(),
            access_mode: config.value.access_mode,
            whitelist: config.value.whitelist,
            blacklist: config.value.blacklist,
            whitelist_added_at: config.value.whitelist_added_at,
            blacklist_added_at: config.value.blacklist_added_at,
        };
        const a = document.createElement("a");
        a.href = URL.createObjectURL(new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" }));
        a.download = "domain-egress-rules.json";
        a.click();
        URL.revokeObjectURL(a.href);
        notify("规则已导出");
    }
    function importRules(event: Event) {
        const input = event.target as HTMLInputElement;
        const file = input.files?.[0];
        if (!file)
            return;
        const reader = new FileReader();
        reader.onload = () => {
            try {
                const x = JSON.parse(String(reader.result));
                if (x.format !== "domain-egress-rules" ||
                    x.version !== 1 ||
                    !Array.isArray(x.whitelist) ||
                    !Array.isArray(x.blacklist))
                    throw new Error("文件格式不正确");
                if (![...x.whitelist, ...x.blacklist].every((r: unknown) => typeof r === "string" && r.trim() && r.length <= 253))
                    throw new Error("文件包含无效规则");
                config.value = {
                    ...config.value,
                    access_mode: x.access_mode === "blacklist" ? "blacklist" : "whitelist",
                    whitelist: [...new Set(x.whitelist as string[])],
                    blacklist: [...new Set(x.blacklist as string[])],
                    whitelist_added_at: x.whitelist_added_at || {},
                    blacklist_added_at: x.blacklist_added_at || {},
                };
                notify("规则已导入草稿，请保存配置后生效");
            }
            catch (e) {
                notify(`导入失败：${e instanceof Error ? e.message : e}`, true);
            }
            finally {
                input.value = "";
            }
        };
        reader.readAsText(file);
    }
    async function refresh(initial = false) {
        if (!desktop)
            return;
        try {
            const result = await call<Snapshot>("snapshot");
            if (!initialized.value) {
                config.value = structuredClone(result.config);
                saved.value = structuredClone(result.config);
                initialized.value = true;
            }
            running.value = result.running;
            logs.value = result.logs;
            traffic.value = result.traffic;
            interfaces.value = result.interfaces ?? [];
            hostname.value = result.hostname ?? "本机";
            sshForwardPorts.value = result.ssh_forward_ports ?? {};
            sshRunning.value = result.ssh_running ?? false;
            sshLocalPort.value = result.ssh_local_port ?? null;
            connected.value = true;
            if (result.message)
                notify(result.message, true);
        }
        catch (e) {
            if (connected.value || initial)
                notify(String(e), true);
            connected.value = false;
        }
    }
    async function loadAutostart() {
        if (!desktop)
            return;
        try {
            autostartEnabled.value = await isEnabled();
        }
        catch (e) {
            notify(`读取开机启动状态失败：${e}`, true);
        }
    }
    async function toggleAutostart() {
        if (!desktop || autostartBusy.value)
            return;
        autostartBusy.value = true;
        try {
            if (autostartEnabled.value)
                await disable();
            else
                await enable();
            autostartEnabled.value = await isEnabled();
            notify(autostartEnabled.value ? "已开启开机启动" : "已关闭开机启动");
        }
        catch (e) {
            notify(`设置开机启动失败：${e}`, true);
        }
        finally {
            autostartBusy.value = false;
        }
    }
    async function action(fn: () => Promise<void>) {
        busy.value = true;
        try {
            await fn();
        }
        catch (e) {
            notify(String(e), true);
        }
        finally {
            busy.value = false;
        }
    }
    async function persistConfig(payload: Config) {
        await action(async () => {
            await call("save_config", { config: payload });
            saved.value = payload;
            notify("配置已保存，访问策略立即生效");
            await refresh();
        });
    }
    async function save() {
        const payload: Config = JSON.parse(JSON.stringify(config.value));
        if (payload.access_mode !== saved.value.access_mode) {
            pendingSave.value = payload;
            confirmAction.value = "save-mode";
            return;
        }
        await persistConfig(payload);
    }
    async function toggle() {
        await action(async () => {
            await call("set_running", { running: !running.value });
            await refresh();
            notify(running.value ? "代理已启动" : "代理已停止");
        });
    }
    function discard() {
        config.value = JSON.parse(JSON.stringify(saved.value));
    }
    function addRules() {
        const items = draft.value
            .split(/[\s,;，；]+/)
            .map((x) => x.trim().toLowerCase().replace(/\.$/, ""))
            .filter(Boolean);
        if (!items.length)
            return;
        const invalid = items.find((item) => {
            if (item.includes(":"))
                return item !== "::" && /^[0-9a-f:]+$/i.test(item);
            return !/^((\*\.)?([a-z0-9-]+\.)+[a-z]{2,}|\.([a-z0-9-]+\.)+[a-z]{2,}|(\d{1,3}\.){3}\d{1,3}|localhost)$/i.test(item);
        });
        if (invalid) {
            notify(`规则格式不正确：${invalid}`, true);
            return;
        }
        let added = 0;
        for (const item of items) {
            if (!activeRules.value.includes(item)) {
                activeRules.value.push(item);
                timestamps.value[item] = Math.floor(Date.now() / 1000);
                added++;
            }
        }
        draft.value = "";
        notify("规则已添加到草稿，点击“保存配置”后生效");
    }
    function removeRule(rule: string) {
        undoRule.value = { rules: [rule], mode: config.value.access_mode };
        config.value[config.value.access_mode] = activeRules.value.filter((r) => r !== rule);
        delete timestamps.value[rule];
        notify(`已删除 ${rule}，可撤销`);
    }
    function undoLastRule() {
        if (!undoRule.value)
            return;
        const { rules, mode } = undoRule.value;
        if (mode === config.value.access_mode)
            config.value[mode] = [...new Set([...rules, ...config.value[mode]])];
        undoRule.value = null;
        notify("已撤销上次规则操作");
    }
    function mainDomain(host: string) {
        if (/^\d+(\.\d+){3}$/.test(host) || host === "localhost")
            return host;
        const parts = host.split(".").filter(Boolean);
        if (parts.length <= 2)
            return host;
        const compoundSuffixes = new Set([
            "co.uk",
            "org.uk",
            "ac.uk",
            "gov.uk",
            "com.cn",
            "net.cn",
            "org.cn",
            "com.hk",
            "co.jp",
            "com.au",
            "co.nz",
            "co.kr",
            "co.in",
            "com.br",
        ]);
        const suffix = parts.slice(-2).join(".");
        return parts.slice(-(compoundSuffixes.has(suffix) ? 3 : 2)).join(".");
    }
    function showTargetMenu(event: MouseEvent, target: string) {
        const host = target
            .trim()
            .toLowerCase()
            .replace(/^https?:\/\//, "")
            .split(/[/?#]/)[0]
            .replace(/\.$/, "");
        const parts = host.split(".").filter(Boolean);
        const candidates = [host];
        if (parts.length > 2 && !/^\d+(\.\d+){3}$/.test(host)) {
            for (let i = 1; i < parts.length - 1; i++)
                candidates.push(`*.${parts.slice(i).join(".")}`);
            candidates.push(mainDomain(host));
        }
        const available = [...new Set(candidates)].filter((rule) => !activeRules.value.includes(rule));
        selectedTargets.value = available.slice(0, 1);
        contextMenu.value = {
            target: host,
            x: event.clientX,
            y: event.clientY,
            candidates: available,
        };
    }
    function addTargetRules() {
        if (!contextMenu.value)
            return;
        const rules = selectedTargets.value.filter((rule) => contextMenu.value?.candidates.includes(rule));
        const now = Math.floor(Date.now() / 1000);
        for (const rule of rules) {
            if (!activeRules.value.includes(rule)) {
                activeRules.value.push(rule);
                timestamps.value[rule] = now;
            }
        }
        notify(rules.length
            ? `${rules.length} 条规则已加入${modeName.value}草稿，请保存配置后生效`
            : "请选择至少一条规则");
        if (rules.length)
            contextMenu.value = null;
    }
    async function clearLogs() {
        await action(async () => {
            await call("clear_logs");
            logs.value = [];
            notify("访问日志已清空");
        });
    }
    watch([logs, autoScrollLogs], async () => {
        if (!autoScrollLogs.value)
            return;
        await nextTick();
        if (logPanel.value)
            logPanel.value.scrollTop = logPanel.value.scrollHeight;
    }, { deep: true });
    async function copyRules() {
        try {
            await navigator.clipboard.writeText(activeRules.value.join("\n"));
            notify("规则已复制");
        }
        catch (e) {
            notify(`复制失败：${e}`, true);
        }
    }
    async function loadPorts() {
        if (portBusy.value)
            return;
        portBusy.value = true;
        try {
            ports.value = await call<PortRow[]>("list_ports");
            portLoaded.value = true;
        }
        catch (e) {
            notify(String(e), true);
        }
        finally {
            portBusy.value = false;
        }
    }
    function navigate(id: string) {
        tab.value = id;
        if (id === "ports" && desktop && !portLoaded.value)
            void loadPorts();
        if (id === "cloud")
            void loadCloudAccounts();
        if (id === "local-dns") {
            void loadHostGroups();
            void loadSystemHosts();
        }
    }
    function addSshProfile() {
        const id = `ssh-${Date.now()}`;
        config.value.ssh_profiles.push({
            id,
            name: `SSH 链路 ${config.value.ssh_profiles.length + 1}`,
            hops: [
                { host: "", port: 22, username: "", auth: "agent", keychain_id: null },
            ],
            enabled: false,
        });
        config.value.active_ssh_profile = id;
    }
    function addSshForward() {
        showSshCommandParser.value = true;
        sshCommand.value = "";
        parsedForwardIds.value = [];
        const id = `forward-${Date.now()}`;
        config.value.ssh_forwards.push({
            id,
            name: `端口转发 ${config.value.ssh_forwards.length + 1}`,
            project: "默认项目",
            note: "",
            local_port: null,
            remote_host: "127.0.0.1",
            remote_port: 3306,
            bind_host: "127.0.0.1",
            ssh_host: "",
            ssh_port: 22,
            ssh_username: "",
            ssh_keychain_id: null,
            auto_start: false,
            ssh_options: [],
        });
    }
    function shellTokens(command: string) {
        const tokens: string[] = [];
        let token = "", quote = "";
        for (let i = 0; i < command.length; i++) {
            const char = command[i];
            if (quote) {
                if (char === quote)
                    quote = "";
                else
                    token += char;
            }
            else if (char === "'" || char === '"')
                quote = char;
            else if (/\s/.test(char)) {
                if (token) {
                    tokens.push(token);
                    token = "";
                }
            }
            else
                token += char;
        }
        if (token)
            tokens.push(token);
        return tokens;
    }
    function parseSshForwardCommand() {
        const tokens = shellTokens(sshCommand.value.trim());
        if (!tokens.length) {
            notify("请先粘贴 SSH 命令", true);
            return;
        }
        const forwards: Array<{
            local: number;
            host: string;
            remote: number;
            bind: string;
        }> = [];
        let sshPort = 22;
        let target = "";
        let i = 0;
        while (i < tokens.length) {
            const token = tokens[i];
            if (token === "-L" || token === "--local-forward") {
                const value = tokens[++i];
                if (value) {
                    const parts = value.split(":");
                    const offset = parts.length === 3 ? 0 : parts.length === 4 ? 1 : -1;
                    if (offset >= 0) {
                        const local = Number(parts[offset]);
                        const host = parts[offset + 1];
                        const remote = Number(parts[offset + 2]);
                        if (local > 0 &&
                            local <= 65535 &&
                            remote > 0 &&
                            remote <= 65535 &&
                            host)
                            forwards.push({
                                local,
                                host,
                                remote,
                                bind: offset ? parts[0] : "127.0.0.1",
                            });
                    }
                }
            }
            else if (token === "-p" || token === "--port") {
                const value = tokens[++i];
                const parsed = Number(value);
                if (parsed > 0 && parsed <= 65535)
                    sshPort = parsed;
            }
            else if (token.startsWith("-L")) {
                const value = token.slice(2);
                if (value) {
                    const parts = value.split(":");
                    const offset = parts.length === 3 ? 0 : parts.length === 4 ? 1 : -1;
                    if (offset >= 0) {
                        const local = Number(parts[offset]);
                        const host = parts[offset + 1];
                        const remote = Number(parts[offset + 2]);
                        if (local > 0 &&
                            local <= 65535 &&
                            remote > 0 &&
                            remote <= 65535 &&
                            host)
                            forwards.push({
                                local,
                                host,
                                remote,
                                bind: offset ? parts[0] : "127.0.0.1",
                            });
                    }
                }
            }
            else if (!token.startsWith("-") && i > 0)
                target = token;
            i++;
        }
        if (!forwards.length) {
            notify("未解析到有效的 -L 本地端口转发", true);
            return;
        }
        const [username, host = ""] = target.includes("@")
            ? target.split("@")
            : ["", target];
        if (!host) {
            notify("未解析到 SSH 服务器，例如 user@example.com", true);
            return;
        }
        const options = tokens
            .flatMap((token, index) => token === "-o" && tokens[index + 1] ? [tokens[index + 1]] : [])
            .filter((option) => option.includes("="));
        const project = host.split(".")[0] || "默认项目";
        const now = Date.now();
        const parsed = forwards.map((item, index) => ({
            id: `forward-${now}-${index}`,
            name: `${item.host}:${item.remote}`,
            project,
            note: "由 SSH 命令解析",
            local_port: item.local,
            remote_host: item.host,
            remote_port: item.remote,
            bind_host: item.bind,
            ssh_host: host,
            ssh_port: sshPort,
            ssh_username: username,
            ssh_keychain_id: null,
            auto_start: false,
            ssh_options: options,
        }));
        config.value.ssh_forwards.push(...parsed);
        parsedForwardIds.value = parsed.map((item) => item.id);
        sshCommand.value = "";
        notify(`已解析 ${forwards.length} 条端口转发，请检查配置后测试并保存`);
    }
    async function testParsedForwards() {
        if (!parsedForwardIds.value.length) {
            notify("请先解析 SSH 命令", true);
            return;
        }
        const payload: Config = JSON.parse(JSON.stringify(config.value));
        await persistConfig(payload);
        await action(async () => {
            for (const id of parsedForwardIds.value) {
                const port = await call<number>("ssh_forward_start", { id });
                sshForwardPorts.value[id] = port;
            }
            await refresh();
            notify(`已保存并启动 ${parsedForwardIds.value.length} 条转发，连接测试通过`);
        });
        parsedForwardIds.value = [];
    }
    async function testSshForward(rule: SshForwardRule) {
        const payload: Config = JSON.parse(JSON.stringify(config.value));
        await persistConfig(payload);
        await action(async () => {
            const port = await call<number>("ssh_forward_start", { id: rule.id });
            sshForwardPorts.value[rule.id] = port;
            await refresh();
            notify(`${rule.name} 测试通过，本地端口 ${port} 已启动`);
        });
    }
    async function testAllSshForwards() {
        if (!config.value.ssh_forwards.length) {
            notify("暂无可测试的端口转发", true);
            return;
        }
        const payload: Config = JSON.parse(JSON.stringify(config.value));
        await persistConfig(payload);
        await action(async () => {
            let success = 0;
            for (const rule of config.value.ssh_forwards) {
                try {
                    const port = await call<number>("ssh_forward_start", { id: rule.id });
                    sshForwardPorts.value[rule.id] = port;
                    success++;
                }
                catch (e) {
                    notify(`${rule.name} 测试失败：${e}`, true);
                }
            }
            await refresh();
            if (success)
                notify(`批量测试完成：${success}/${config.value.ssh_forwards.length} 条转发已启动`);
        });
    }
    function removeSshForward(id: string) {
        config.value.ssh_forwards = config.value.ssh_forwards.filter((rule) => rule.id !== id);
    }
    function sshOptionsText(rule: SshForwardRule) {
        return rule.ssh_options.join("\n");
    }
    function setSshOptions(rule: SshForwardRule, event: Event) {
        rule.ssh_options = (event.target as HTMLTextAreaElement).value
            .split("\n")
            .map((item) => item.trim())
            .filter(Boolean);
    }
    async function toggleSshForward(rule: SshForwardRule) {
        await action(async () => {
            if (sshForwardPorts.value[rule.id]) {
                await call("ssh_forward_stop", { id: rule.id });
                delete sshForwardPorts.value[rule.id];
                notify(`${rule.name} 已停止`);
            }
            else {
                const port = await call<number>("ssh_forward_start", { id: rule.id });
                sshForwardPorts.value[rule.id] = port;
                rule.local_port = port;
                notify(`${rule.name} 已启动，本地端口 ${port}`);
            }
        });
    }
    function removeSshProfile(id: string) {
        config.value.ssh_profiles = config.value.ssh_profiles.filter((profile) => profile.id !== id);
        if (config.value.active_ssh_profile === id)
            config.value.active_ssh_profile = config.value.ssh_profiles[0]?.id ?? null;
    }
    function addSshHop(profile: SshProfile) {
        profile.hops.push({
            host: "",
            port: 22,
            username: "",
            auth: "agent",
            keychain_id: null,
        });
    }
    function removeSshHop(profile: SshProfile, index: number) {
        if (profile.hops.length > 1)
            profile.hops.splice(index, 1);
    }
    function moveSshHop(profile: SshProfile, from: number, to: number) {
        if (from === to || to < 0 || to >= profile.hops.length)
            return;
        const [hop] = profile.hops.splice(from, 1);
        profile.hops.splice(to, 0, hop);
    }
    function beginHopDrag(profile: SshProfile, index: number) {
        draggedHop.value = { profile, index };
    }
    function dropHop(profile: SshProfile, index: number) {
        if (draggedHop.value?.profile === profile)
            moveSshHop(profile, draggedHop.value.index, index);
        draggedHop.value = null;
    }
    async function syncIcloud() {
        await action(async () => {
            const path = await call<string>("icloud_sync");
            config.value.icloud_sync_enabled = true;
            notify(`配置已同步到 iCloud：${path}`);
        });
    }
    async function mergeIcloud() {
        await action(async () => {
            const remote = await call<Config | null>("icloud_read");
            if (!remote) {
                notify("iCloud 中暂无配置");
                return;
            }
            config.value = {
                ...config.value,
                ...remote,
                whitelist: [...new Set([...config.value.whitelist, ...remote.whitelist])],
                blacklist: [...new Set([...config.value.blacklist, ...remote.blacklist])],
                ssh_profiles: [
                    ...config.value.ssh_profiles,
                    ...remote.ssh_profiles.filter((r) => !config.value.ssh_profiles.some((l) => l.id === r.id)),
                ],
            };
            notify("iCloud 配置已合并到草稿，请检查后保存");
        });
    }
    async function pullGist() {
        await action(async () => {
            const remote = await call<{
                whitelist: string[];
                blacklist: string[];
            }>("gist_pull", {
                provider: config.value.gist_provider,
                gist_id: config.value.gist_id,
                file_name: config.value.gist_file_name,
                token: gistToken.value,
            });
            config.value.whitelist = [
                ...new Set([...config.value.whitelist, ...remote.whitelist]),
            ];
            config.value.blacklist = [
                ...new Set([...config.value.blacklist, ...remote.blacklist]),
            ];
            notify("Gist 规则已合并到草稿，请检查后保存");
        });
    }
    async function pushGist() {
        await action(async () => {
            await call("gist_push", {
                provider: config.value.gist_provider,
                gist_id: config.value.gist_id,
                file_name: config.value.gist_file_name,
                token: gistToken.value,
                config: config.value,
            });
            gistToken.value = "";
            notify("当前规则已推送到 Gist");
        });
    }
    async function saveSshSecret(hop: {
        keychain_id?: string | null;
    }, key: string) {
        const secret = sshSecrets.value[key];
        if (!secret || !hop.keychain_id) {
            notify("请先填写钥匙串标识和凭据", true);
            return;
        }
        await action(async () => {
            await call("keychain_set", { account: hop.keychain_id, secret });
            sshSecrets.value[key] = "";
            notify("SSH 凭据已保存到 macOS 钥匙串");
        });
    }
    async function deleteSshSecret(hop: {
        keychain_id?: string | null;
    }) {
        if (!hop.keychain_id)
            return;
        await action(async () => {
            await call("keychain_delete", { account: hop.keychain_id });
            notify("SSH 凭据已从 macOS 钥匙串删除");
        });
    }
    async function checkIcloud() {
        if (desktop) {
            try {
                icloudAvailable.value = await call<boolean>("icloud_status");
            }
            catch {
                icloudAvailable.value = false;
            }
        }
    }
    function closeConfirm() {
        confirmAction.value = null;
        pendingSave.value = null;
    }
    async function confirm() {
        const selected = confirmAction.value;
        if (selected === "clear") {
            const previous = [...activeRules.value];
            undoRule.value = { rules: previous, mode: config.value.access_mode };
            config.value[config.value.access_mode] = [];
            config.value[config.value.access_mode === "whitelist"
                ? "whitelist_added_at"
                : "blacklist_added_at"] = {};
            confirmAction.value = null;
            notify(`已清空${modeName.value}草稿，可点击撤销`);
        }
        else if (selected === "save-mode" && pendingSave.value) {
            const payload = pendingSave.value;
            closeConfirm();
            await persistConfig(payload);
        }
        else if (selected && selected !== "save-mode") {
            await action(async () => {
                await call("terminate_process", {
                    pid: selected.pid,
                    started: selected.started,
                });
                confirmAction.value = null;
                notify(`已向 PID ${selected.pid} 发送终止信号`);
                await loadPorts();
            });
        }
    }
    let timer: ReturnType<typeof setTimeout> | undefined;
    let portRefreshTimer: ReturnType<typeof setInterval> | undefined;
    let cloudSyncTimer: ReturnType<typeof setInterval> | undefined;
    let disposed = false;
    async function poll() {
        await refresh();
        if (!disposed)
            timer = setTimeout(poll, 1500);
    }
    function schedulePortRefresh() {
        clearInterval(portRefreshTimer);
        portRefreshTimer = undefined;
        if (tab.value === "ports" && desktop) {
            portRefreshTimer = window.setInterval(() => void loadPorts(), Math.max(1, saved.value.port_refresh_interval_seconds || 30) * 1000);
        }
    }
    watch([tab, () => saved.value.port_refresh_interval_seconds], schedulePortRefresh, { immediate: true });
    onMounted(async () => {
        window.addEventListener("keydown", handleFontScaleShortcut);
        seedCloudPreview();
        await refresh(true);
        await loadAutostart();
        await checkIcloud();
        await loadManagedRules();
        void probePublicIp();
        void checkUpdate();
        if (desktop) {
            void syncAllManagedRules(true);
            cloudSyncTimer = window.setInterval(() => void syncAllManagedRules(true), 5 * 60 * 1000);
        }
        if (!disposed)
            timer = setTimeout(poll, 1500);
    });
    onUnmounted(() => {
        window.removeEventListener("keydown", handleFontScaleShortcut);
        disposed = true;
        clearTimeout(timer);
        clearTimeout(noticeTimer);
        clearInterval(portRefreshTimer);
        clearInterval(cloudSyncTimer);
    });
    return {
        ref,
        AlertTriangle,
        ArrowUpRight,
        Check,
        ChevronLeft,
        ChevronRight,
        CircleHelp,
        CircleStop,
        Cloud,
        Copy,
        FileKey2,
        Globe2,
        ListFilter,
        Network,
        Play,
        Plus,
        Radio,
        RefreshCw,
        Search,
        Server,
        ShieldCheck,
        Square,
        Trash2,
        X,
        appearance,
        theme,
        themes,
        appearanceError,
        setAppearance,
        setTheme,
        desktop,
        tabs,
        tab,
        config,
        saved,
        pageTitle,
        pageEyebrow,
        pageDescription,
        running,
        logs,
        busy,
        connected,
        initialized,
        sshRunning,
        sshLocalPort,
        icloudAvailable,
        interfaces,
        hostname,
        publicIpBusy,
        cloudAccounts,
        cloudLoading,
        cloudAccountDialog,
        cloudSubtab,
        cloudAccountForm,
        cloudRegions,
        cloudSecurityGroups,
        cloudRegionLoading,
        cloudGroupsLoading,
        selectedCloudAccountId,
        selectedCloudRegion,
        selectedSecurityGroupId,
        cloudSecurityRules,
        cloudRulesLoading,
        managedRules,
        managedRuleDialog,
        managedRuleForm,
        managedRuleSourceCidr,
        managedRuleSourceLoading,
        managedRuleSourceError,
        sshForwardPorts,
        sshCommand,
        showSshCommandParser,
        parsedForwardIds,
        updateInfo,
        updateBusy,
        updateDismissed,
        notice,
        error,
        draft,
        search,
        logLevel,
        logOutcome,
        ruleSort,
        ruleSearch,
        trendRange,
        trendInterval,
        autoScrollLogs,
        undoRule,
        ports,
        portBusy,
        portSearch,
        confirmAction,
        contextMenu,
        selectedTargets,
        gistToken,
        logPanel,
        fontScaleChoices,
        dirty,
        showGlobalOptions,
        autostartEnabled,
        autostartBusy,
        expandedRuleOptions,
        networkTool,
        networkProbe,
        networkResult,
        networkBusy,
        hostsLoading,
        hostGroups,
        selectedDnsGroup,
        hostGroupContent,
        selectedDnsGroupDirty,
        systemHostsContent,
        selectedDnsView,
        hostGroupNameDraft,
        systemEditorGutter,
        systemEditorCode,
        groupEditorGutter,
        groupEditorCode,
        systemEditorLines,
        groupEditorLines,
        isHostsComment,
        syncHostsEditorScroll,
        networkTools,
        optionParts,
        addGlobalOption,
        updateGlobalOption,
        removeGlobalOption,
        selectNetworkTool,
        detailText,
        runNetworkProbe,
        cancelNetworkProbe,
        loadHostGroups,
        loadSystemHosts,
        selectSystemHosts,
        selectDnsGroup,
        saveDnsGroup,
        toggleDnsGroup,
        createDnsGroup,
        deleteDnsGroup,
        toggleRuleOptions,
        addRuleOption,
        updateRuleOption,
        removeRuleOption,
        modeName,
        activeRules,
        timestamps,
        orderedRules,
        filteredLogs,
        blockedDomains,
        filteredPorts,
        selectedDnsEntries,
        groupedSshForwards,
        isForwardGroupExpanded,
        toggleForwardGroup,
        selectedForwardGroup,
        forwardMenuCollapsed,
        selectedForwardRules,
        selectForwardGroup,
        localAddressSummary,
        egressProbes,
        buckets,
        maximum,
        trendRangeLabel,
        admitted,
        blocked,
        date,
        ruleDate,
        adjustFontScale,
        handleCopyClick,
        locationLabel,
        probePublicIp,
        loadCloudAccounts,
        openCloudAccountDialog,
        saveCloudAccount,
        verifyCloudAccount,
        loadCloudRegions,
        loadCloudSecurityGroups,
        selectCloudSecurityGroup,
        loadCloudSecurityRules,
        refreshManagedRuleSource,
        openManagedRuleDialog,
        createManagedRule,
        syncManagedRule,
        syncAllManagedRules,
        deleteManagedRule,
        deleteCloudAccount,
        checkUpdate,
        openUpdate,
        refresh,
        toggleAutostart,
        save,
        toggle,
        discard,
        addRules,
        removeRule,
        undoLastRule,
        showTargetMenu,
        addTargetRules,
        clearLogs,
        copyRules,
        loadPorts,
        navigate,
        addSshProfile,
        addSshForward,
        parseSshForwardCommand,
        testParsedForwards,
        testSshForward,
        testAllSshForwards,
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
        closeConfirm,
        confirm
    };
}
