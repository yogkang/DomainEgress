<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import {
  Activity,
  AlertTriangle,
  ArrowUpRight,
  Check,
  ChevronLeft,
  ChevronRight,
  CircleHelp,
  CircleStop,
  Cloud,
  Copy,
  Download,
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
  Settings2,
  ShieldCheck,
  Square,
  Trash2,
  Upload,
  Wifi,
  X,
} from "lucide-vue-next";
import {
  appearance,
  theme,
  themes,
  appearanceError,
  setAppearance,
  setTheme,
} from "./theme";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import {
  call,
  defaults,
  desktop,
  type CloudAccount,
  type CloudRegion,
  type CloudSecurityGroup,
  type CloudSecurityRule,
  type Config,
  type CreateManagedRuleInput,
  type EgressAddress,
  type HostGroup,
  type HostMapping,
  type LogEntry,
  type ManagedRuleConfig,
  type ManagedRuleSyncResult,
  type NetworkInterface,
  type NetworkProbeRequest,
  type NetworkProbeResult,
  type NetworkTool,
  type PortRow,
  type PublicIpProbe,
  type SaveCloudAccountInput,
  type Snapshot,
  type SshForwardRule,
  type SshProfile,
  type UpdateInfo,
} from "./api";
const tabs = [
  { id: "overview", title: "代理概览", icon: Activity },
  { id: "rules", title: "访问控制", icon: ShieldCheck },
  { id: "logs", title: "访问日志", icon: ListFilter },
  { id: "ports", title: "监听端口", icon: Network },
  { id: "ssh-forward", title: "SSH 端口转发", icon: Network },
  { id: "network-tools", title: "网络探测", icon: Radio },
  { id: "local-dns", title: "本地 DNS", icon: Server },
  { id: "settings", title: "应用设置", icon: Settings2 },
];
const tab = ref("overview"),
  config = ref<Config>(structuredClone(defaults)),
  saved = ref<Config>(structuredClone(defaults));
const pageTitle = computed(() =>
  tab.value === "cloud"
    ? "云资源"
    : tabs.find((item) => item.id === tab.value)?.title || "",
);
const pageEyebrow = computed(
  () =>
    ({
      overview: "NETWORK OVERVIEW",
      rules: "ACCESS POLICY",
      logs: "REQUEST LOGS",
      ports: "SYSTEM NETWORK",
      "ssh-forward": "SSH PORT FORWARDING",
      "network-tools": "NETWORK DIAGNOSTICS",
      "local-dns": "LOCAL HOSTS",
      cloud: "CLOUD RESOURCES",
      settings: "PREFERENCES",
    })[tab.value] || "WORKSPACE",
);
const pageDescription = computed(
  () =>
    ({
      overview: "让每一次网络访问，都在掌控之中。",
      rules: "定义允许或拒绝访问的域名与 IP 地址。",
      logs: "查看本次运行的访问决策与请求信息。",
      ports: "查看本机 TCP 监听端口及所属进程。",
      "ssh-forward": "将远程服务器端口安全映射到本机访问。",
      "network-tools": "从本机直接验证目标的可达性、协议和证书。",
      "local-dns": "维护当前 Mac 的应用专属 /etc/hosts 映射。",
      cloud: "按云账号管理远程资源与网络访问能力。",
      settings: "管理代理监听地址、启动行为与日志策略。",
    })[tab.value] || "",
);
const running = ref(false),
  logs = ref<LogEntry[]>([]),
  traffic = ref<number[]>([]),
  busy = ref(false),
  connected = ref(!desktop),
  initialized = ref(false),
  sshRunning = ref(false),
  sshLocalPort = ref<number | null>(null),
  icloudAvailable = ref(false);
const interfaces = ref<NetworkInterface[]>([]);
const hostname = ref("本机");
const emptyEgressProbe = (): PublicIpProbe["system"] => ({
  addresses: [],
  confidence: "未探测",
  error: null,
});
const minimumSpinnerDuration = 420;
const publicIp = ref<PublicIpProbe>({ system: emptyEgressProbe(), ipv6: null }),
  publicIpBusy = ref(false);
const cloudAccounts = ref<CloudAccount[]>([]),
  cloudLoading = ref(false),
  cloudAccountDialog = ref(false),
  cloudSubtab = ref<"accounts" | "security-groups">("accounts");
const cloudAccountForm = ref<SaveCloudAccountInput>({
  display_name: "",
  auth_method: "ram_access_key",
  access_key_id: "",
  access_key_secret: "",
});
const cloudRegions = ref<CloudRegion[]>([]),
  cloudSecurityGroups = ref<CloudSecurityGroup[]>([]),
  cloudRegionLoading = ref(false),
  cloudGroupsLoading = ref(false),
  selectedCloudAccountId = ref(""),
  selectedCloudRegion = ref("");
const selectedSecurityGroupId = ref(""),
  cloudSecurityRules = ref<CloudSecurityRule[]>([]),
  cloudRulesLoading = ref(false),
  managedRules = ref<ManagedRuleConfig[]>([]),
  managedRuleDialog = ref(false);
const managedRuleForm = ref<CreateManagedRuleInput>({
  account_id: "",
  region: "",
  security_group_id: "",
  protocol: "TCP",
  port_range: "22/22",
  priority: 1,
});
const managedRuleSourceCidr = ref(""),
  managedRuleSourceLoading = ref(false),
  managedRuleSourceError = ref("");
let cloudRegionRequest = 0,
  cloudGroupRequest = 0,
  cloudRuleRequest = 0,
  managedSourceRequest = 0;
let cloudSyncRunning = false;
const sshForwardPorts = ref<Record<string, number>>({});
const sshCommand = ref("");
const showSshCommandParser = ref(false),
  parsedForwardIds = ref<string[]>([]);
const updateInfo = ref<UpdateInfo | null>(null),
  updateBusy = ref(false),
  updateDismissed = ref(false);
const notice = ref(""),
  error = ref(false),
  draft = ref(""),
  search = ref(""),
  logLevel = ref("all"),
  logOutcome = ref("all"),
  ruleSort = ref("name"),
  ruleSearch = ref(""),
  trendRange = ref(60),
  trendInterval = ref(60),
  autoScrollLogs = ref(true),
  trendStart = ref(""),
  trendEnd = ref(""),
  undoRule = ref<{ rules: string[]; mode: "whitelist" | "blacklist" } | null>(
    null,
  );
const ports = ref<PortRow[]>([]),
  portBusy = ref(false),
  portLoaded = ref(false),
  portSearch = ref(""),
  confirmAction = ref<"clear" | "save-mode" | PortRow | null>(null),
  pendingSave = ref<Config | null>(null),
  contextMenu = ref<{
    target: string;
    x: number;
    y: number;
    candidates: string[];
  } | null>(null),
  selectedTargets = ref<string[]>([]),
  sshSecrets = ref<Record<string, string>>({}),
  gistToken = ref(""),
  draggedHop = ref<{ profile: SshProfile; index: number } | null>(null);
const logPanel = ref<HTMLElement | null>(null);
const fontScaleChoices = [90, 100, 110, 125];
const dirty = computed(
  () => JSON.stringify(config.value) !== JSON.stringify(saved.value),
);
const showGlobalOptions = ref(false);
const autostartEnabled = ref(false),
  autostartBusy = ref(false);
const expandedRuleOptions = ref<Record<string, boolean>>({});
const networkTool = ref<NetworkTool>("ping");
const networkProbe = ref<NetworkProbeRequest>({
  tool: "ping",
  target: "example.com",
  port: null,
  timeout_ms: 3000,
  count: 4,
  method: "GET",
  headers: "",
  body: "",
  payload: "",
  payload_hex: false,
});
const networkResult = ref<NetworkProbeResult | null>(null),
  networkBusy = ref(false);
const hostMappings = ref<HostMapping[]>([]),
  hostsLoading = ref(false);
const hostForm = ref({ ip: "", domains: "", group: "" });
const hostBatchText = ref("");
const hostBatchGroup = ref("");
const hostGroups = ref<HostGroup[]>([]);
const selectedDnsGroup = ref("");
const hostGroupContent = ref("");
const systemHostsContent = ref("");
const selectedDnsView = ref<"system" | "group">("system");
const hostGroupNameDraft = ref("");
const selectedHostGroup = ref("");
const editingHostKey = ref("");
const editingHostOriginal = ref<HostMapping | null>(null);
const editingHostForm = ref({ ip: "", domains: "", group: "" });
const systemEditorGutter = ref<HTMLElement | null>(null);
const systemEditorCode = ref<HTMLElement | null>(null);
const groupEditorGutter = ref<HTMLElement | null>(null);
const groupEditorCode = ref<HTMLElement | null>(null);
const systemEditorLines = computed(() => systemHostsContent.value.split("\n"));
const groupEditorLines = computed(() => hostGroupContent.value.split("\n"));
function isHostsComment(line: string) {
  return /^\s*#/.test(line);
}
function syncHostsEditorScroll(
  event: Event,
  gutter: HTMLElement | null,
  code: HTMLElement | null,
) {
  const editor = event.currentTarget as HTMLTextAreaElement;
  if (gutter) gutter.scrollTop = editor.scrollTop;
  if (code) {
    code.scrollTop = editor.scrollTop;
    code.scrollLeft = editor.scrollLeft;
  }
}
const networkTools = [
  { id: "ping", title: "Ping", icon: Radio },
  { id: "telnet", title: "Telnet / TCP", icon: Server },
  { id: "certificate", title: "域名证书", icon: FileKey2 },
  { id: "traceroute", title: "Traceroute", icon: Network },
  { id: "tcp", title: "TCP 客户端", icon: Wifi },
  { id: "udp", title: "UDP 客户端", icon: Wifi },
  { id: "http", title: "HTTP 客户端", icon: Globe2 },
  { id: "websocket", title: "WebSocket", icon: Radio },
] as { id: NetworkTool; title: string; icon: typeof Radio }[];
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
function selectNetworkTool(tool: NetworkTool) {
  networkTool.value = tool;
  networkProbe.value.tool = tool;
  if (tool === "http" && !networkProbe.value.target.includes("://"))
    networkProbe.value.target = `https://${networkProbe.value.target}`;
  if (tool === "websocket" && !networkProbe.value.target.startsWith("ws"))
    networkProbe.value.target = `wss://${networkProbe.value.target}`;
}
function detailText(value: unknown) {
  return typeof value === "string" ? value : JSON.stringify(value, null, 2);
}
async function runNetworkProbe() {
  if (!desktop) {
    notify("请在 DomainEgress 桌面应用中执行真实网络探测", true);
    return;
  }
  networkBusy.value = true;
  networkResult.value = null;
  const request = { ...networkProbe.value, tool: networkTool.value };
  try {
    networkResult.value = await call<NetworkProbeResult>("run_network_probe", {
      request,
    });
  } catch (e) {
    notify(String(e), true);
  } finally {
    networkBusy.value = false;
  }
}
async function cancelNetworkProbe() {
  if (!desktop) return;
  try {
    await call("cancel_network_probe");
  } catch (e) {
    notify(String(e), true);
  }
}
async function loadHostMappings() {
  if (!desktop) return;
  hostsLoading.value = true;
  try {
    hostMappings.value = await call<HostMapping[]>("list_host_mappings");
  } catch (e) {
    notify(String(e), true);
  } finally {
    hostsLoading.value = false;
  }
}
async function loadHostGroups() {
  if (!desktop) return;
  hostsLoading.value = true;
  try {
    hostGroups.value = await call<HostGroup[]>("list_host_groups");
    const group =
      hostGroups.value.find((item) => item.name === selectedDnsGroup.value) ||
      hostGroups.value[0];
    if (group) selectDnsGroup(group.name);
  } catch (e) {
    notify(String(e), true);
  } finally {
    hostsLoading.value = false;
  }
}
async function loadSystemHosts() {
  if (!desktop) return;
  try {
    systemHostsContent.value = await call<string>("read_system_hosts");
  } catch (e) {
    notify(String(e), true);
  }
}
function selectSystemHosts() {
  selectedDnsView.value = "system";
}
function selectDnsGroup(name: string) {
  const group = hostGroups.value.find((item) => item.name === name);
  if (!group) return;
  selectedDnsView.value = "group";
  selectedDnsGroup.value = name;
  hostGroupContent.value = group.content;
}
async function saveDnsGroup() {
  const group = hostGroups.value.find(
    (item) => item.name === selectedDnsGroup.value,
  );
  if (!group) return;
  await action(async () => {
    hostGroups.value = await call<HostGroup[]>("save_host_group", {
      name: group.name,
      content: hostGroupContent.value,
      enabled: group.enabled,
    });
    notify("本地 DNS 分组已保存，DNS 缓存已刷新");
  });
}
async function toggleDnsGroup(group: HostGroup) {
  await action(async () => {
    hostGroups.value = await call<HostGroup[]>("save_host_group", {
      name: group.name,
      content:
        group.name === selectedDnsGroup.value
          ? hostGroupContent.value
          : group.content,
      enabled: !group.enabled,
    });
    if (group.name === selectedDnsGroup.value) selectDnsGroup(group.name);
    notify(`${group.name} 分组已${group.enabled ? "关闭" : "开启"}`);
  });
}
async function createDnsGroup() {
  const name = hostGroupNameDraft.value.trim();
  if (!name) {
    notify("请输入分组名称", true);
    return;
  }
  if (hostGroups.value.some((group) => group.name === name)) {
    notify("分组名称已存在", true);
    return;
  }
  await action(async () => {
    hostGroups.value = await call<HostGroup[]>("save_host_group", {
      name,
      content: "",
      enabled: true,
    });
    hostGroupNameDraft.value = "";
    selectDnsGroup(name);
    notify("本地 DNS 分组已创建");
  });
}
async function deleteDnsGroup(group: HostGroup) {
  if (!window.confirm(`删除分组“${group.name}”及其中的全部映射？`)) return;
  await action(async () => {
    hostGroups.value = await call<HostGroup[]>("delete_host_group", {
      name: group.name,
    });
    selectedDnsGroup.value = "";
    hostGroupContent.value = "";
    const next = hostGroups.value[0];
    if (next) selectDnsGroup(next.name);
    notify("本地 DNS 分组已删除，DNS 缓存已刷新");
  });
}
async function addHostMapping() {
  const ip = hostForm.value.ip.trim();
  const domains = hostForm.value.domains
    .split(/[\s,;，；]+/)
    .map((item) => item.trim())
    .filter(Boolean);
  if (!ip || !domains.length) {
    notify("请填写 IP 和至少一个域名", true);
    return;
  }
  await action(async () => {
    hostMappings.value = await call<HostMapping[]>("add_host_mapping", {
      ip,
      domains,
      group: hostForm.value.group.trim(),
    });
    hostForm.value = { ip: "", domains: "", group: "" };
    notify("本地 hosts 映射已添加，DNS 缓存已刷新");
  });
}
function parseHostBatch(text: string) {
  const byIp = new Map<string, string[]>();
  const errors: string[] = [];
  text.split(/\r?\n/).forEach((line, index) => {
    const value = line.replace(/^\s*#\s?/, "").trim();
    if (!value) return;
    const tokens = value.split(/\s+/).filter(Boolean);
    let ip = "";
    let domains: string[] = [];
    const looksLikeIp = (token: string) =>
      /^[0-9a-fA-F:.]+$/.test(token) &&
      (token.includes(".") || token.includes(":"));
    if (tokens.length >= 2 && looksLikeIp(tokens[0])) {
      ip = tokens[0];
      domains = tokens.slice(1);
    } else {
      const match = value.match(/^([^\s=,:]+)\s*(?:=|:)\s*(\S+)$/);
      if (match) {
        ip = match[2];
        domains = [match[1]];
      } else if (tokens.length >= 2 && looksLikeIp(tokens.at(-1) || "")) {
        ip = tokens.at(-1) || "";
        domains = tokens.slice(0, -1);
      }
    }
    if (!ip || !domains.length) {
      errors.push(`第 ${index + 1} 行格式无法识别`);
      return;
    }
    const current = byIp.get(ip) || [];
    byIp.set(ip, [
      ...current,
      ...domains.filter((domain) => !current.includes(domain)),
    ]);
  });
  return {
    mappings: [...byIp].map(([ip, domains]) => ({ ip, domains })),
    errors,
  };
}
function toggleHostBatchComments(event: KeyboardEvent) {
  if (!(event.ctrlKey || event.metaKey) || event.key !== "/") return;
  event.preventDefault();
  const textarea = event.currentTarget as HTMLTextAreaElement;
  const value = textarea.value;
  const start =
    value.lastIndexOf("\n", Math.max(0, textarea.selectionStart - 1)) + 1;
  const nextBreak = value.indexOf("\n", textarea.selectionEnd);
  const end = nextBreak === -1 ? value.length : nextBreak;
  const selected = value.slice(start, end);
  const lines = selected.split("\n");
  const contentLines = lines.filter((line) => line.trim());
  const uncomment =
    contentLines.length > 0 && contentLines.every((line) => /^\s*#/.test(line));
  const updated = lines
    .map((line) => {
      if (!line.trim()) return line;
      if (uncomment) return line.replace(/^(\s*)#\s?/, "$1");
      return line.replace(/^(\s*)/, "$1# ");
    })
    .join("\n");
  hostBatchText.value = value.slice(0, start) + updated + value.slice(end);
  void nextTick(() =>
    textarea.setSelectionRange(start, start + updated.length),
  );
}
async function addHostBatch() {
  const { mappings, errors } = parseHostBatch(hostBatchText.value);
  if (errors.length) {
    notify(errors.join("；"), true);
    return;
  }
  if (!mappings.length) {
    notify("请粘贴至少一行映射", true);
    return;
  }
  await action(async () => {
    let result = hostMappings.value;
    for (const mapping of mappings)
      result = await call<HostMapping[]>("add_host_mapping", {
        ...mapping,
        group: hostBatchGroup.value.trim(),
      });
    hostMappings.value = result;
    hostBatchText.value = "";
    hostBatchGroup.value = "";
    notify(
      `已添加 ${mappings.length} 个地址、${mappings.reduce((count, mapping) => count + mapping.domains.length, 0)} 个域名，DNS 缓存已刷新`,
    );
  });
}
function hostMappingKey(mapping: HostMapping) {
  return `${mapping.ip}\u0000${mapping.domains.join(",")}`;
}
function beginEditHostMapping(mapping: HostMapping) {
  editingHostKey.value = hostMappingKey(mapping);
  editingHostOriginal.value = { ...mapping, domains: [...mapping.domains] };
  editingHostForm.value = {
    ip: mapping.ip,
    domains: mapping.domains.join(" "),
    group: mapping.group,
  };
}
function cancelEditHostMapping() {
  editingHostKey.value = "";
  editingHostOriginal.value = null;
}
async function saveHostMappingEdit() {
  const original = editingHostOriginal.value;
  const ip = editingHostForm.value.ip.trim();
  const domains = editingHostForm.value.domains
    .split(/[\s,;，；]+/)
    .map((item) => item.trim())
    .filter(Boolean);
  if (!original || !ip || !domains.length) {
    notify("请填写 IP 和至少一个域名", true);
    return;
  }
  await action(async () => {
    hostMappings.value = await call<HostMapping[]>("update_host_mapping", {
      old_ip: original.ip,
      old_domains: original.domains,
      ip,
      domains,
      group: editingHostForm.value.group.trim(),
    });
    selectedHostGroup.value = editingHostForm.value.group.trim() || "未分组";
    cancelEditHostMapping();
    notify("本地 hosts 映射已修改，DNS 缓存已刷新");
  });
}
async function removeHostMapping(mapping: HostMapping) {
  if (
    !window.confirm(
      `删除 ${mapping.ip} 的 ${mapping.domains.join(", ")} 映射？`,
    )
  )
    return;
  await action(async () => {
    hostMappings.value = await call<HostMapping[]>("remove_host_mapping", {
      ip: mapping.ip,
      domains: mapping.domains,
    });
    notify("本地 hosts 映射已删除，DNS 缓存已刷新");
  });
}
function toggleRuleOptions(id: string) {
  expandedRuleOptions.value[id] = !expandedRuleOptions.value[id];
}
function addRuleOption(rule: SshForwardRule) {
  rule.ssh_options.push("NewOption=");
  expandedRuleOptions.value[rule.id] = true;
}
function updateRuleOption(
  rule: SshForwardRule,
  index: number,
  key: string,
  value: string,
) {
  rule.ssh_options[index] = `${key.trim()}=${value}`;
}
function removeRuleOption(rule: SshForwardRule, index: number) {
  rule.ssh_options.splice(index, 1);
}
const modeName = computed(() =>
  config.value.access_mode === "whitelist" ? "白名单" : "黑名单",
);
const activeRules = computed(() => config.value[config.value.access_mode]);
const timestamps = computed(
  () =>
    config.value[
      config.value.access_mode === "whitelist"
        ? "whitelist_added_at"
        : "blacklist_added_at"
    ],
);
const orderedRules = computed(() =>
  [...activeRules.value]
    .filter((r) => r.includes(ruleSearch.value.trim().toLowerCase()))
    .sort((a, b) =>
      ruleSort.value === "name"
        ? a.localeCompare(b)
        : (timestamps.value[b] || 0) - (timestamps.value[a] || 0),
    ),
);
const filteredLogs = computed(() =>
  logs.value
    .filter(
      (l) =>
        (logLevel.value === "all" || logLevel.value === l.level) &&
        (logOutcome.value === "all" || logOutcome.value === l.outcome) &&
        Object.values(l)
          .join(" ")
          .toLowerCase()
          .includes(search.value.toLowerCase()),
    )
    .reverse(),
);
const range = computed(() => {
  const end = Date.now() / 1000;
  return { start: end - trendRange.value * 60, end };
});
const blockedLogs = computed(() =>
  logs.value.filter(
    (l) =>
      l.outcome === "拦截" &&
      l.timestamp >= range.value.start &&
      l.timestamp <= range.value.end,
  ),
);
const blockedDomains = computed(() => {
  const grouped = new Map<string, LogEntry & { count: number }>();
  for (const log of blockedLogs.value) {
    const key = `${log.target}\u0000${log.source}`;
    const item = grouped.get(key);
    if (item) item.count += 1;
    else grouped.set(key, { ...log, count: 1 });
  }
  return [...grouped.values()].sort(
    (a, b) => b.count - a.count || b.timestamp - a.timestamp,
  );
});
const filteredPorts = computed(() =>
  ports.value.filter((p) =>
    Object.values(p)
      .join(" ")
      .toLowerCase()
      .includes(portSearch.value.toLowerCase()),
  ),
);
const hostMappingGroups = computed(() => {
  const groups = new Map<string, HostMapping[]>();
  for (const mapping of hostMappings.value) {
    const name = mapping.group.trim() || "未分组";
    groups.set(name, [...(groups.get(name) || []), mapping]);
  }
  return [...groups].map(([name, mappings]) => ({ name, mappings }));
});
const selectedHostMappings = computed(
  () =>
    hostMappingGroups.value.find(
      (group) => group.name === selectedHostGroup.value,
    )?.mappings || [],
);
const selectedDnsEntries = computed(() => {
  if (selectedDnsView.value !== "group") return [];
  const entries = hostGroupContent.value.split(/\r?\n/).flatMap((line, index) => {
    const value = line.replace(/^\s*#\s?/, "").trim();
    if (!value || value.startsWith("#")) return [];
    const [ip, ...domains] = value.split(/\s+/);
    return ip && domains.length ? [{ line: index + 1, ip, domains }] : [];
  });
  const domainCounts = new Map<string, number>();
  entries.forEach((entry) => entry.domains.forEach((domain) => domainCounts.set(domain.toLowerCase(), (domainCounts.get(domain.toLowerCase()) || 0) + 1)));
  return entries.map((entry) => ({ ...entry, duplicate: entry.domains.some((domain) => (domainCounts.get(domain.toLowerCase()) || 0) > 1) }));
});
watch(
  hostMappingGroups,
  (groups) => {
    if (!groups.some((group) => group.name === selectedHostGroup.value))
      selectedHostGroup.value = groups[0]?.name || "";
  },
  { immediate: true },
);
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
const selectedForwardRules = computed(
  () =>
    groupedSshForwards.value.find(
      (group) => group.name === selectedForwardGroup.value,
    )?.rules || [],
);
function selectForwardGroup(name: string) {
  selectedForwardGroup.value = selectedForwardGroup.value === name ? "" : name;
}
watch(
  groupedSshForwards,
  (groups) => {
    if (!groups.some((group) => group.name === selectedForwardGroup.value))
      selectedForwardGroup.value = groups.at(-1)?.name || "";
  },
  { immediate: true },
);
const localAddressSummary = computed(
  () =>
    [
      ...new Set(
        interfaces.value
          .flatMap((item) => item.addresses)
          .filter((address) => !address.includes(":")),
      ),
    ].join(" · ") || "未检测到",
);
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
      count: traffic.value.filter(
        (t) =>
          t >= bucket &&
          t < bucket + step &&
          t >= range.value.start &&
          t <= range.value.end,
      ).length,
    };
  });
});
const maximum = computed(() =>
  Math.max(1, ...buckets.value.map((b) => b.count)),
);
const trendRangeLabel = computed(() =>
  trendRange.value < 60
    ? `最近 ${trendRange.value} 分钟`
    : `最近 ${trendRange.value / 60} 小时`,
);
const admitted = computed(() => buckets.value.reduce((n, b) => n + b.count, 0));
const blocked = computed(
  () => logs.value.filter((l) => l.outcome === "拦截").length,
);
const date = (t: number) =>
  t
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
  const next = Math.max(
    0,
    Math.min(
      fontScaleChoices.length - 1,
      (current < 0 ? 1 : current) + direction,
    ),
  );
  config.value.font_scale = fontScaleChoices[next];
}
function resetFontScale() {
  config.value.font_scale = 100;
}
function handleFontScaleShortcut(event: KeyboardEvent) {
  if (!event.ctrlKey || event.altKey || event.metaKey) return;
  if (event.key === "+" || event.key === "=" || event.code === "NumpadAdd") {
    event.preventDefault();
    adjustFontScale(1);
  } else if (event.key === "-" || event.code === "NumpadSubtract") {
    event.preventDefault();
    adjustFontScale(-1);
  } else if (event.key === "0" || event.code === "Numpad0") {
    event.preventDefault();
    resetFontScale();
  }
}
async function handleCopyClick(event: MouseEvent) {
  contextMenu.value = null;
  const target = event.target as HTMLElement;
  if (target.closest("button, input, textarea, select, a")) return;
  const element = target.closest(
    ".interface-address, code, .mono, .service-endpoints span, .public-ip-result strong",
  ) as HTMLElement | null;
  const value = element?.innerText?.trim();
  if (!value || !desktop) return;
  try {
    await navigator.clipboard.writeText(value);
    notify(`已复制：${value}`);
  } catch {
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
  } catch (e) {
    publicIp.value = {
      system: {
        ...emptyEgressProbe(),
        confidence: "探测失败",
        error: String(e),
      },
      ipv6: null,
    };
  } finally {
    await keepSpinnerVisible(startedAt);
    publicIpBusy.value = false;
  }
}
async function loadCloudAccounts() {
  if (!desktop) return;
  cloudLoading.value = true;
  try {
    cloudAccounts.value = await call<CloudAccount[]>("list_cloud_accounts");
  } catch (e) {
    notify(String(e), true);
  } finally {
    cloudLoading.value = false;
  }
}
function openCloudAccountDialog() {
  cloudAccountForm.value = {
    display_name: "",
    auth_method: "ram_access_key",
    access_key_id: "",
    access_key_secret: "",
  };
  cloudAccountDialog.value = true;
}
async function saveCloudAccount() {
  const input = {
    ...cloudAccountForm.value,
    display_name: cloudAccountForm.value.display_name.trim(),
    access_key_id: cloudAccountForm.value.access_key_id.trim(),
    access_key_secret: cloudAccountForm.value.access_key_secret.trim(),
  };
  await action(async () => {
    const account = await call<CloudAccount>("save_cloud_account", { input });
    cloudAccountDialog.value = false;
    await loadCloudAccounts();
    notify(
      `已验证并保存阿里云账号：${account.verified_account_id || account.display_name}`,
    );
  });
}
async function verifyCloudAccount(account: CloudAccount) {
  await action(async () => {
    await call<CloudAccount>("verify_cloud_account", { id: account.id });
    await loadCloudAccounts();
    notify(`阿里云账号已验证：${account.display_name}`);
  });
}
async function loadCloudRegions() {
  const request = ++cloudRegionRequest,
    accountId = selectedCloudAccountId.value;
  selectedCloudRegion.value = "";
  selectedSecurityGroupId.value = "";
  cloudRegions.value = [];
  cloudSecurityGroups.value = [];
  cloudSecurityRules.value = [];
  if (!accountId) return;
  cloudRegionLoading.value = true;
  try {
    const regions = await call<CloudRegion[]>("list_cloud_regions", {
      accountId,
    });
    if (
      request !== cloudRegionRequest ||
      accountId !== selectedCloudAccountId.value
    )
      return;
    cloudRegions.value = regions;
    selectedCloudRegion.value = regions[0]?.id || "";
    if (selectedCloudRegion.value) await loadCloudSecurityGroups();
  } catch (e) {
    if (request === cloudRegionRequest) notify(String(e), true);
  } finally {
    if (request === cloudRegionRequest) cloudRegionLoading.value = false;
  }
}
async function loadCloudSecurityGroups() {
  const request = ++cloudGroupRequest,
    accountId = selectedCloudAccountId.value,
    region = selectedCloudRegion.value;
  selectedSecurityGroupId.value = "";
  cloudSecurityGroups.value = [];
  cloudSecurityRules.value = [];
  if (!accountId || !region) return;
  cloudGroupsLoading.value = true;
  try {
    const groups = await call<CloudSecurityGroup[]>(
      "list_cloud_security_groups",
      { accountId, region },
    );
    if (request !== cloudGroupRequest || region !== selectedCloudRegion.value)
      return;
    cloudSecurityGroups.value = groups;
  } catch (e) {
    if (request === cloudGroupRequest) notify(String(e), true);
  } finally {
    if (request === cloudGroupRequest) cloudGroupsLoading.value = false;
  }
}
async function selectCloudSecurityGroup(group: CloudSecurityGroup) {
  selectedSecurityGroupId.value = group.id;
  await loadCloudSecurityRules();
}
async function loadCloudSecurityRules() {
  const request = ++cloudRuleRequest,
    accountId = selectedCloudAccountId.value,
    region = selectedCloudRegion.value,
    securityGroupId = selectedSecurityGroupId.value;
  cloudSecurityRules.value = [];
  if (!accountId || !region || !securityGroupId) return;
  cloudRulesLoading.value = true;
  try {
    const rules = await call<CloudSecurityRule[]>(
      "list_cloud_security_group_rules",
      { accountId, region, securityGroupId },
    );
    if (
      request !== cloudRuleRequest ||
      securityGroupId !== selectedSecurityGroupId.value
    )
      return;
    cloudSecurityRules.value = rules;
  } catch (e) {
    if (request === cloudRuleRequest) notify(String(e), true);
  } finally {
    if (request === cloudRuleRequest) cloudRulesLoading.value = false;
  }
}
async function loadManagedRules() {
  if (!desktop) return;
  try {
    managedRules.value = await call<ManagedRuleConfig[]>(
      "list_cloud_managed_rules",
    );
  } catch (e) {
    notify(String(e), true);
  }
}
async function refreshManagedRuleSource() {
  const request = ++managedSourceRequest;
  managedRuleSourceCidr.value = "";
  managedRuleSourceError.value = "";
  if (!desktop) {
    managedRuleSourceCidr.value = "203.0.113.8/32";
    return;
  }
  managedRuleSourceLoading.value = true;
  try {
    const cidr = await call<string>("preview_cloud_managed_source");
    if (request === managedSourceRequest) managedRuleSourceCidr.value = cidr;
  } catch (e) {
    if (request === managedSourceRequest)
      managedRuleSourceError.value = String(e);
  } finally {
    if (request === managedSourceRequest)
      managedRuleSourceLoading.value = false;
  }
}
function openManagedRuleDialog() {
  managedRuleForm.value = {
    account_id: selectedCloudAccountId.value,
    region: selectedCloudRegion.value,
    security_group_id: selectedSecurityGroupId.value,
    protocol: "TCP",
    port_range: "22/22",
    priority: 1,
  };
  managedRuleDialog.value = true;
  void refreshManagedRuleSource();
}
async function createManagedRule() {
  await action(async () => {
    const result = await call<ManagedRuleSyncResult>(
      "create_cloud_managed_rule",
      { input: managedRuleForm.value },
    );
    managedRuleDialog.value = false;
    await Promise.all([loadManagedRules(), loadCloudSecurityRules()]);
    notify(
      `受管规则已${result.action === "created" ? "创建" : "更新"}：${result.config.last_source_cidr}`,
    );
  });
  await loadManagedRules();
}
async function syncManagedRule(id: string) {
  await action(async () => {
    const result = await call<ManagedRuleSyncResult>(
      "sync_cloud_managed_rule",
      { id },
    );
    await Promise.all([loadManagedRules(), loadCloudSecurityRules()]);
    notify(`受管规则已同步到 ${result.config.last_source_cidr}`);
  });
}
async function syncAllManagedRules(silent = false) {
  if (
    !desktop ||
    cloudSyncRunning ||
    !managedRules.value.some((rule) => rule.enabled)
  )
    return;
  cloudSyncRunning = true;
  try {
    const results = await call<ManagedRuleSyncResult[]>(
      "sync_all_cloud_managed_rules",
    );
    await loadManagedRules();
    if (selectedSecurityGroupId.value) await loadCloudSecurityRules();
    if (!silent && results.length)
      notify(`已同步 ${results.length} 条受管规则`);
  } catch (e) {
    await loadManagedRules();
    if (!silent) notify(String(e), true);
  } finally {
    cloudSyncRunning = false;
  }
}
async function deleteManagedRule(rule: ManagedRuleConfig) {
  if (
    !window.confirm(
      `删除受管规则“${rule.description}”？对应的阿里云规则也会被撤销。`,
    )
  )
    return;
  busy.value = true;
  try {
    await call("delete_cloud_managed_rule", {
      id: rule.id,
      revokeRemote: true,
    });
    await Promise.all([loadManagedRules(), loadCloudSecurityRules()]);
    notify("受管规则及对应云端规则已删除");
  } catch (e) {
    const force = window.confirm(
      `${String(e)}\n\n是否仅移除本机管理记录？这可能在阿里云中留下仍开放的规则，请稍后到云控制台核对。`,
    );
    if (force) {
      try {
        await call("delete_cloud_managed_rule", {
          id: rule.id,
          revokeRemote: false,
        });
        await loadManagedRules();
        notify(
          "已仅移除本机管理记录；请到阿里云控制台核对并清理可能残留的规则",
          true,
        );
      } catch (localError) {
        notify(String(localError), true);
      }
    } else {
      notify(String(e), true);
    }
  } finally {
    busy.value = false;
  }
}
function seedCloudPreview() {
  if (desktop) return;
  cloudAccounts.value = [
    {
      id: "preview-account",
      provider: "aliyun",
      display_name: "生产 RAM 子账户",
      auth_method: "ram_access_key",
      access_key_hint: "LTAI****8A2F",
      verified_account_id: "1234567890123456",
      verification_status: "已验证",
      created_at: 0,
      updated_at: 0,
    },
  ];
  selectedCloudAccountId.value = "preview-account";
  cloudRegions.value = [{ id: "cn-hangzhou", name: "华东 1（杭州）" }];
  selectedCloudRegion.value = "cn-hangzhou";
  cloudSecurityGroups.value = [
    {
      id: "sg-bp1d3x-preview",
      name: "Web-Production",
      vpc_id: "vpc-bp1-preview",
      group_type: "normal",
    },
  ];
  selectedSecurityGroupId.value = "sg-bp1d3x-preview";
  cloudSecurityRules.value = [
    {
      id: "sgr-managed",
      direction: "ingress",
      protocol: "TCP",
      port_range: "22/22",
      priority: 1,
      action: "Accept",
      source_cidr: "203.0.113.8/32",
      description: "DomainEgress:v1:preview-rule",
      managed: true,
    },
    {
      id: "sgr-https",
      direction: "ingress",
      protocol: "TCP",
      port_range: "443/443",
      priority: 1,
      action: "Accept",
      source_cidr: "0.0.0.0/0",
      description: "HTTPS",
      managed: false,
    },
  ];
  managedRules.value = [
    {
      id: "preview-rule",
      account_id: "preview-account",
      region: "cn-hangzhou",
      security_group_id: "sg-bp1d3x-preview",
      protocol: "TCP",
      port_range: "22/22",
      priority: 1,
      description: "DomainEgress:v1:preview-rule",
      enabled: true,
      last_source_cidr: "203.0.113.8/32",
      last_synced_at: 0,
      last_error: null,
    },
  ];
}
async function deleteCloudAccount(account: CloudAccount) {
  if (
    !window.confirm(
      `删除云账号“${account.display_name}”？将移除本机保存的凭据。`,
    )
  )
    return;
  await action(async () => {
    await call("delete_cloud_account", { id: account.id });
    await loadCloudAccounts();
    notify(`已删除云账号：${account.display_name}`);
  });
}
async function checkUpdate() {
  if (!desktop) return;
  updateBusy.value = true;
  const startedAt = performance.now();
  await nextTick();
  try {
    updateInfo.value = await call<UpdateInfo>("check_update");
  } catch (e) {
    updateInfo.value = {
      current_version: "0.4.1",
      latest_version: null,
      release_url: null,
      available: false,
      error: String(e),
    };
  } finally {
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
  a.href = URL.createObjectURL(
    new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" }),
  );
  a.download = "domain-egress-rules.json";
  a.click();
  URL.revokeObjectURL(a.href);
  notify("规则已导出");
}
function importRules(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  const reader = new FileReader();
  reader.onload = () => {
    try {
      const x = JSON.parse(String(reader.result));
      if (
        x.format !== "domain-egress-rules" ||
        x.version !== 1 ||
        !Array.isArray(x.whitelist) ||
        !Array.isArray(x.blacklist)
      )
        throw new Error("文件格式不正确");
      if (
        ![...x.whitelist, ...x.blacklist].every(
          (r: unknown) => typeof r === "string" && r.trim() && r.length <= 253,
        )
      )
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
    } catch (e) {
      notify(`导入失败：${e instanceof Error ? e.message : e}`, true);
    } finally {
      input.value = "";
    }
  };
  reader.readAsText(file);
}
async function refresh(initial = false) {
  if (!desktop) return;
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
    if (result.message) notify(result.message, true);
  } catch (e) {
    if (connected.value || initial) notify(String(e), true);
    connected.value = false;
  }
}
async function loadAutostart() {
  if (!desktop) return;
  try {
    autostartEnabled.value = await isEnabled();
  } catch (e) {
    notify(`读取开机启动状态失败：${e}`, true);
  }
}
async function toggleAutostart() {
  if (!desktop || autostartBusy.value) return;
  autostartBusy.value = true;
  try {
    if (autostartEnabled.value) await disable();
    else await enable();
    autostartEnabled.value = await isEnabled();
    notify(autostartEnabled.value ? "已开启开机启动" : "已关闭开机启动");
  } catch (e) {
    notify(`设置开机启动失败：${e}`, true);
  } finally {
    autostartBusy.value = false;
  }
}
async function action(fn: () => Promise<void>) {
  busy.value = true;
  try {
    await fn();
  } catch (e) {
    notify(String(e), true);
  } finally {
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
  if (!items.length) return;
  const invalid = items.find((item) => {
    if (item.includes(":")) return item !== "::" && /^[0-9a-f:]+$/i.test(item);
    return !/^((\*\.)?([a-z0-9-]+\.)+[a-z]{2,}|\.([a-z0-9-]+\.)+[a-z]{2,}|(\d{1,3}\.){3}\d{1,3}|localhost)$/i.test(
      item,
    );
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
  config.value[config.value.access_mode] = activeRules.value.filter(
    (r) => r !== rule,
  );
  delete timestamps.value[rule];
  notify(`已删除 ${rule}，可撤销`);
}
function undoLastRule() {
  if (!undoRule.value) return;
  const { rules, mode } = undoRule.value;
  if (mode === config.value.access_mode)
    config.value[mode] = [...new Set([...rules, ...config.value[mode]])];
  undoRule.value = null;
  notify("已撤销上次规则操作");
}
function mainDomain(host: string) {
  if (/^\d+(\.\d+){3}$/.test(host) || host === "localhost") return host;
  const parts = host.split(".").filter(Boolean);
  if (parts.length <= 2) return host;
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
  const available = [...new Set(candidates)].filter(
    (rule) => !activeRules.value.includes(rule),
  );
  selectedTargets.value = available.slice(0, 1);
  contextMenu.value = {
    target: host,
    x: event.clientX,
    y: event.clientY,
    candidates: available,
  };
}
function addTargetRules() {
  if (!contextMenu.value) return;
  const rules = selectedTargets.value.filter((rule) =>
    contextMenu.value?.candidates.includes(rule),
  );
  const now = Math.floor(Date.now() / 1000);
  for (const rule of rules) {
    if (!activeRules.value.includes(rule)) {
      activeRules.value.push(rule);
      timestamps.value[rule] = now;
    }
  }
  notify(
    rules.length
      ? `${rules.length} 条规则已加入${modeName.value}草稿，请保存配置后生效`
      : "请选择至少一条规则",
  );
  if (rules.length) contextMenu.value = null;
}
async function clearLogs() {
  await action(async () => {
    await call("clear_logs");
    logs.value = [];
    notify("访问日志已清空");
  });
}
watch(
  [logs, autoScrollLogs],
  async () => {
    if (!autoScrollLogs.value) return;
    await nextTick();
    if (logPanel.value) logPanel.value.scrollTop = logPanel.value.scrollHeight;
  },
  { deep: true },
);
async function copyRules() {
  try {
    await navigator.clipboard.writeText(activeRules.value.join("\n"));
    notify("规则已复制");
  } catch (e) {
    notify(`复制失败：${e}`, true);
  }
}
async function loadPorts() {
  if (portBusy.value) return;
  portBusy.value = true;
  try {
    ports.value = await call<PortRow[]>("list_ports");
    portLoaded.value = true;
  } catch (e) {
    notify(String(e), true);
  } finally {
    portBusy.value = false;
  }
}
function navigate(id: string) {
  tab.value = id;
  if (id === "ports" && desktop && !portLoaded.value) void loadPorts();
  if (id === "cloud") void loadCloudAccounts();
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
  let token = "",
    quote = "";
  for (let i = 0; i < command.length; i++) {
    const char = command[i];
    if (quote) {
      if (char === quote) quote = "";
      else token += char;
    } else if (char === "'" || char === '"') quote = char;
    else if (/\s/.test(char)) {
      if (token) {
        tokens.push(token);
        token = "";
      }
    } else token += char;
  }
  if (token) tokens.push(token);
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
          if (
            local > 0 &&
            local <= 65535 &&
            remote > 0 &&
            remote <= 65535 &&
            host
          )
            forwards.push({
              local,
              host,
              remote,
              bind: offset ? parts[0] : "127.0.0.1",
            });
        }
      }
    } else if (token === "-p" || token === "--port") {
      const value = tokens[++i];
      const parsed = Number(value);
      if (parsed > 0 && parsed <= 65535) sshPort = parsed;
    } else if (token.startsWith("-L")) {
      const value = token.slice(2);
      if (value) {
        const parts = value.split(":");
        const offset = parts.length === 3 ? 0 : parts.length === 4 ? 1 : -1;
        if (offset >= 0) {
          const local = Number(parts[offset]);
          const host = parts[offset + 1];
          const remote = Number(parts[offset + 2]);
          if (
            local > 0 &&
            local <= 65535 &&
            remote > 0 &&
            remote <= 65535 &&
            host
          )
            forwards.push({
              local,
              host,
              remote,
              bind: offset ? parts[0] : "127.0.0.1",
            });
        }
      }
    } else if (!token.startsWith("-") && i > 0) target = token;
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
    .flatMap((token, index) =>
      token === "-o" && tokens[index + 1] ? [tokens[index + 1]] : [],
    )
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
    notify(
      `已保存并启动 ${parsedForwardIds.value.length} 条转发，连接测试通过`,
    );
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
      } catch (e) {
        notify(`${rule.name} 测试失败：${e}`, true);
      }
    }
    await refresh();
    if (success)
      notify(
        `批量测试完成：${success}/${config.value.ssh_forwards.length} 条转发已启动`,
      );
  });
}
function removeSshForward(id: string) {
  config.value.ssh_forwards = config.value.ssh_forwards.filter(
    (rule) => rule.id !== id,
  );
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
    } else {
      const port = await call<number>("ssh_forward_start", { id: rule.id });
      sshForwardPorts.value[rule.id] = port;
      rule.local_port = port;
      notify(`${rule.name} 已启动，本地端口 ${port}`);
    }
  });
}
function removeSshProfile(id: string) {
  config.value.ssh_profiles = config.value.ssh_profiles.filter(
    (profile) => profile.id !== id,
  );
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
  if (profile.hops.length > 1) profile.hops.splice(index, 1);
}
function moveSshHop(profile: SshProfile, from: number, to: number) {
  if (from === to || to < 0 || to >= profile.hops.length) return;
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
        ...remote.ssh_profiles.filter(
          (r) => !config.value.ssh_profiles.some((l) => l.id === r.id),
        ),
      ],
    };
    notify("iCloud 配置已合并到草稿，请检查后保存");
  });
}
async function pullGist() {
  await action(async () => {
    const remote = await call<{ whitelist: string[]; blacklist: string[] }>(
      "gist_pull",
      {
        provider: config.value.gist_provider,
        gist_id: config.value.gist_id,
        file_name: config.value.gist_file_name,
        token: gistToken.value,
      },
    );
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
async function saveSshSecret(
  hop: { keychain_id?: string | null },
  key: string,
) {
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
async function deleteSshSecret(hop: { keychain_id?: string | null }) {
  if (!hop.keychain_id) return;
  await action(async () => {
    await call("keychain_delete", { account: hop.keychain_id });
    notify("SSH 凭据已从 macOS 钥匙串删除");
  });
}
async function checkIcloud() {
  if (desktop) {
    try {
      icloudAvailable.value = await call<boolean>("icloud_status");
    } catch {
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
    config.value[
      config.value.access_mode === "whitelist"
        ? "whitelist_added_at"
        : "blacklist_added_at"
    ] = {};
    confirmAction.value = null;
    notify(`已清空${modeName.value}草稿，可点击撤销`);
  } else if (selected === "save-mode" && pendingSave.value) {
    const payload = pendingSave.value;
    closeConfirm();
    await persistConfig(payload);
  } else if (selected && selected !== "save-mode") {
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
  if (!disposed) timer = setTimeout(poll, 1500);
}
function schedulePortRefresh() {
  clearInterval(portRefreshTimer);
  portRefreshTimer = undefined;
  if (tab.value === "ports" && desktop) {
    portRefreshTimer = window.setInterval(
      () => void loadPorts(),
      Math.max(1, saved.value.port_refresh_interval_seconds || 30) * 1000,
    );
  }
}
watch(
  [tab, () => saved.value.port_refresh_interval_seconds],
  schedulePortRefresh,
  { immediate: true },
);
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
    cloudSyncTimer = window.setInterval(
      () => void syncAllManagedRules(true),
      5 * 60 * 1000,
    );
  }
  if (!disposed) timer = setTimeout(poll, 1500);
});
onUnmounted(() => {
  window.removeEventListener("keydown", handleFontScaleShortcut);
  disposed = true;
  clearTimeout(timer);
  clearTimeout(noticeTimer);
  clearInterval(portRefreshTimer);
  clearInterval(cloudSyncTimer);
});
</script>

<template>
  <div
    class="app-shell"
    :style="{ '--font-scale': config.font_scale / 100 }"
    @click="handleCopyClick"
  >
    <aside class="sidebar">
      <div class="brand">
        <span class="brand-mark"><Globe2 :size="24" /></span>
        <div>DomainEgress<small>本地网络访问控制</small></div>
      </div>
      <div class="nav-label">工作空间</div>
      <nav aria-label="主导航">
        <button
          v-for="item in tabs.filter((item) => item.id !== 'settings')"
          :key="item.id"
          :class="{ active: tab === item.id }"
          @click="navigate(item.id)"
        >
          <component :is="item.icon" :size="18" /><span>{{ item.title }}</span
          ><ChevronRight v-if="tab === item.id" :size="15" />
        </button>
        <button
          class="cloud-nav-parent"
          :class="{ active: tab === 'cloud' }"
          @click="navigate('cloud')"
        >
          <Cloud :size="18" /><span>云资源</span><ChevronRight :size="15" />
        </button>
        <div v-if="tab === 'cloud'" class="cloud-nav-children">
          <button
            :class="{ active: cloudSubtab === 'accounts' }"
            @click="cloudSubtab = 'accounts'"
          >
            云账号
            <small v-if="cloudAccounts.length">{{
              cloudAccounts.length
            }}</small>
          </button>
          <button
            :class="{ active: cloudSubtab === 'security-groups' }"
            @click="cloudSubtab = 'security-groups'"
          >
            安全组
          </button>
        </div>
        <button
          v-for="item in tabs.filter((item) => item.id === 'settings')"
          :key="item.id"
          :class="{ active: tab === item.id }"
          @click="navigate(item.id)"
        >
          <component :is="item.icon" :size="18" /><span>{{ item.title }}</span
          ><ChevronRight v-if="tab === item.id" :size="15" />
        </button>
      </nav>
      <div class="sidebar-bottom">
        <div class="local-badge">
          <span class="dot" :class="{ live: running }"></span
          >{{ running ? "代理正在运行" : "代理已停止" }}
        </div>
        <p>本地网络安全代理</p>
        <span class="version">DESKTOP / 0.4.1</span
        ><button
          class="sidebar-update"
          :disabled="updateBusy || !desktop"
          @click="checkUpdate"
        >
          <RefreshCw
            :size="13"
            :class="['refresh-icon', { 'is-spinning': updateBusy }]"
          />{{ updateBusy ? "检查中…" : "检查更新" }}</button
        ><small v-if="updateInfo?.available" class="sidebar-update-hint"
          >发现新版本 v{{ updateInfo.latest_version }}</small
        ><small
          v-else-if="updateInfo && !updateInfo.error"
          class="sidebar-update-hint"
          >当前已是最新版本</small
        >
      </div>
    </aside>
    <main>
      <header>
        <div class="breadcrumb">
          工作空间 <ChevronRight :size="13" /> <span>{{ pageTitle }}</span>
        </div>
        <div class="header-right">
          <span class="desktop-label">{{
            desktop ? "本机桌面" : "界面预览"
          }}</span
          ><span class="dot" :class="{ live: connected && desktop }"></span
          >{{ desktop ? (connected ? "核心已连接" : "连接中断") : "未连接核心"
          }}<span class="header-separator"></span
          ><span class="host-info" :title="hostname">主机名 {{ hostname }}</span
          ><span class="header-separator"></span
          ><span class="host-info" :title="localAddressSummary"
            >IPv4 {{ localAddressSummary }}</span
          >
        </div>
      </header>
      <div
        class="content"
        :class="{
          'forward-view': tab === 'ssh-forward',
          'forward-menu-collapsed': forwardMenuCollapsed,
        }"
      >
        <div v-if="!desktop" class="preview-banner">
          <CircleHelp :size="17" />
          当前为浏览器界面预览。代理操作、配置保存与端口查询请使用桌面应用。
        </div>
        <div class="page-heading">
          <div>
            <div class="eyebrow">{{ pageEyebrow }}</div>
            <h1>{{ pageTitle }}</h1>
            <p>{{ pageDescription }}</p>
          </div>
          <div class="page-heading-actions">
            <button
              v-if="
                tab === 'rules' ||
                tab === 'settings' ||
                tab === 'ssh-forward' ||
                (tab === 'overview' && dirty)
              "
              class="primary"
              :disabled="
                busy || !desktop || !initialized || !connected || !dirty
              "
              @click="save"
            >
              <Check :size="16" />保存配置<span
                v-if="dirty"
                class="unsaved"
              ></span></button
            ><span v-if="tab === 'overview'" class="pill"
              >HTTP / HTTPS / SOCKS5</span
            >
          </div>
        </div>
        <div v-if="notice" class="notice" :class="{ error }" role="status">
          <span>{{ notice }}</span
          ><button v-if="undoRule" class="undo" @click="undoLastRule">
            撤销</button
          ><button aria-label="关闭提示" @click="notice = ''">
            <X :size="16" />
          </button>
        </div>
        <div v-if="dirty" class="draft-banner">
          有未保存的修改，保存后生效。<button @click="discard">撤销修改</button>
        </div>
        <div
          v-if="updateInfo?.available && !updateDismissed"
          class="update-banner"
        >
          <RefreshCw :size="17" /><span
            >发现新版本 <strong>v{{ updateInfo.latest_version }}</strong
            >，当前版本 v{{ updateInfo.current_version }}。</span
          ><button class="primary" @click="openUpdate">查看更新</button
          ><button
            class="update-dismiss"
            aria-label="稍后提醒"
            @click="updateDismissed = true"
          >
            稍后
          </button>
        </div>

        <template v-if="tab === 'network-tools'">
          <section class="network-tool-layout">
            <aside class="panel network-tool-menu">
              <button
                v-for="tool in networkTools"
                :key="tool.id"
                :class="{ active: networkTool === tool.id }"
                @click="selectNetworkTool(tool.id)"
              >
                <component :is="tool.icon" :size="16" /><span>{{
                  tool.title
                }}</span
                ><ChevronRight :size="14" />
              </button>
            </aside>
            <section class="network-tool-main">
              <section class="panel">
                <div class="section-heading">
                  <div>
                    <h3>
                      {{
                        networkTools.find((item) => item.id === networkTool)
                          ?.title
                      }}
                    </h3>
                    <p>输入目标后从本机执行一次诊断，不会保存请求内容。</p>
                  </div>
                  <div class="toolbar">
                    <span v-if="networkBusy" class="pill">执行中…</span
                    ><button
                      v-if="networkBusy"
                      class="danger-text"
                      @click="cancelNetworkProbe"
                    >
                      <Square :size="14" />取消</button
                    ><button
                      v-else
                      class="primary"
                      :disabled="!desktop"
                      @click="runNetworkProbe"
                    >
                      <Play :size="15" />开始探测
                    </button>
                  </div>
                </div>
                <div class="network-form-grid">
                  <label class="wide"
                    >目标<input
                      v-model="networkProbe.target"
                      :placeholder="
                        networkTool === 'http'
                          ? 'https://example.com'
                          : networkTool === 'websocket'
                            ? 'wss://example.com/socket'
                            : 'example.com 或 IP 地址'
                      "
                  /></label>
                  <label
                    v-if="
                      ['telnet', 'tcp', 'udp', 'certificate'].includes(
                        networkTool,
                      )
                    "
                    >端口<input
                      v-model.number="networkProbe.port"
                      type="number"
                      min="1"
                      max="65535"
                      :placeholder="
                        networkTool === 'certificate' ? '443' : '80'
                      "
                  /></label>
                  <label
                    >超时（毫秒）<input
                      v-model.number="networkProbe.timeout_ms"
                      type="number"
                      min="100"
                      max="120000"
                  /></label>
                  <label v-if="networkTool === 'ping'"
                    >次数<input
                      v-model.number="networkProbe.count"
                      type="number"
                      min="1"
                      max="20"
                  /></label>
                  <label v-if="networkTool === 'http'"
                    >方法<select v-model="networkProbe.method">
                      <option>GET</option>
                      <option>POST</option>
                      <option>PUT</option>
                      <option>DELETE</option>
                      <option>HEAD</option>
                      <option>PATCH</option>
                    </select></label
                  >
                </div>
                <div
                  v-if="['tcp', 'udp', 'websocket'].includes(networkTool)"
                  class="network-extra-fields"
                >
                  <label
                    >发送数据<textarea
                      v-model="networkProbe.payload"
                      rows="3"
                      placeholder="可选；TCP/UDP/WebSocket 首次发送内容"
                    ></textarea>
                  </label>
                  <label class="checkbox-line"
                    ><input
                      v-model="networkProbe.payload_hex"
                      type="checkbox"
                    />按十六进制发送</label
                  >
                </div>
                <div v-if="networkTool === 'http'" class="network-extra-fields">
                  <label
                    >请求头<textarea
                      v-model="networkProbe.headers"
                      rows="3"
                      placeholder="每行一个，例如：Authorization: Bearer …"
                    ></textarea></label
                  ><label
                    >请求体<textarea
                      v-model="networkProbe.body"
                      rows="3"
                      placeholder="可选请求正文"
                    ></textarea>
                  </label>
                </div>
              </section>
              <section
                v-if="networkResult"
                class="panel network-result"
                :class="{ failed: !networkResult.success }"
              >
                <div class="section-heading">
                  <div>
                    <h3>{{ networkResult.summary }}</h3>
                    <p>
                      {{ networkResult.target }} ·
                      {{ networkResult.elapsed_ms }} ms
                    </p>
                  </div>
                  <span
                    class="pill"
                    :class="networkResult.success ? 'green' : 'red'"
                    >{{ networkResult.success ? "成功" : "失败" }}</span
                  >
                </div>
                <p v-if="networkResult.error" class="network-error">
                  {{ networkResult.error }}
                </p>
                <pre>{{ detailText(networkResult.details) }}</pre>
              </section>
              <div v-else class="panel empty network-empty">
                <Radio :size="30" />
                <h3>等待执行探测</h3>
                <p>桌面应用会从当前网络环境直接发起连接。</p>
              </div>
            </section>
          </section>
        </template>
        <template v-else-if="tab === 'local-dns'">
          <section class="panel hosts-editor-panel">
            <div class="section-heading">
              <div>
                <h3>系统 Hosts</h3>
                <p>
                  查看完整的系统 <code>/etc/hosts</code>，或编辑 DomainEgress
                  管理的 hosts 分组。
                </p>
              </div>
              <button
                :disabled="hostsLoading || !desktop"
                @click="loadHostGroups(); loadSystemHosts()"
              >
                <RefreshCw :size="15" :class="{ spin: hostsLoading }" />刷新
              </button>
            </div>
            <div class="hosts-editor-layout">
              <aside class="hosts-group-sidebar">
                <button
                  class="hosts-system-item"
                  :class="{ active: selectedDnsView === 'system' }"
                  @click="selectSystemHosts"
                >
                  <Server :size="17" /><span>系统 Hosts</span>
                </button>
                <div
                  v-for="group in hostGroups"
                  :key="group.name"
                  class="hosts-group-item"
                  :class="{ active: selectedDnsGroup === group.name }"
                >
                  <button
                    class="hosts-group-select"
                    @click="selectDnsGroup(group.name)"
                  >
                    <FileKey2 :size="16" /><span>{{ group.name }}</span></button
                  ><button
                    class="hosts-group-delete"
                    :disabled="busy || !desktop"
                    aria-label="删除分组"
                    @click="deleteDnsGroup(group)"
                  >
                    <Trash2 :size="14" /></button
                  ><button
                    class="switch"
                    :class="{ on: group.enabled }"
                    :aria-label="`${group.name}${group.enabled ? '已开启' : '已关闭'}`"
                    @click="toggleDnsGroup(group)"
                  >
                    <span></span>
                  </button>
                </div>
                <div class="hosts-new-group">
                  <input
                    v-model="hostGroupNameDraft"
                    placeholder="新分组名称"
                  /><button
                    :disabled="busy || !desktop"
                    @click="createDnsGroup"
                  >
                    <Plus :size="15" />添加分组
                  </button>
                </div>
              </aside>
            <section v-if="selectedDnsView === 'system'" class="hosts-editor-surface">
              <div class="hosts-editor-toolbar">
                <div><strong>/etc/hosts</strong><span>只读查看</span></div>
              </div>
              <div class="hosts-code-editor hosts-code-editor-readonly">
                <div ref="systemEditorGutter" class="hosts-line-numbers" aria-hidden="true">
                  <span v-for="(_, index) in systemEditorLines" :key="index">{{ index + 1 }}</span>
                </div>
                <div ref="systemEditorCode" class="hosts-code-layer" aria-hidden="true">
                  <pre class="hosts-code-highlight"><span v-for="(line, index) in systemEditorLines" :key="index" :class="{ comment: isHostsComment(line) }">{{ line || " " }}</span></pre>
                </div>
                <textarea
                  v-model="systemHostsContent"
                  class="hosts-native-editor"
                  readonly
                  spellcheck="false"
                  aria-label="系统 Hosts 内容"
                  @scroll="syncHostsEditorScroll($event, systemEditorGutter, systemEditorCode)"
                ></textarea>
              </div>
              <p class="footnote"><CircleHelp :size="15" />这里显示当前 Mac 的完整系统 Hosts 文件，包含系统条目和 DomainEgress 管理的分组。</p>
            </section>
            <section v-else-if="selectedDnsGroup" class="hosts-editor-surface">
                <div class="hosts-editor-toolbar">
                  <div>
                    <strong>{{ selectedDnsGroup }}</strong
                    ><span>{{
                      hostGroups.find(
                        (group) => group.name === selectedDnsGroup,
                      )?.enabled
                        ? "已开启"
                        : "已关闭"
                    }}</span>
                  </div>
                  <div class="toolbar">
                    <button :disabled="busy || !desktop" @click="saveDnsGroup">
                      <Check :size="15" />保存</button
                    ><button
                      class="danger-text"
                      :disabled="busy || !desktop"
                      @click="
                        deleteDnsGroup(
                          hostGroups.find(
                            (group) => group.name === selectedDnsGroup,
                          )!,
                        )
                      "
                    >
                      <Trash2 :size="15" />删除分组
                    </button>
                  </div>
                </div>
                <div class="hosts-code-editor">
                  <div ref="groupEditorGutter" class="hosts-line-numbers" aria-hidden="true">
                    <span v-for="(_, index) in groupEditorLines" :key="index">{{ index + 1 }}</span>
                  </div>
                  <div ref="groupEditorCode" class="hosts-code-layer" aria-hidden="true">
                    <pre class="hosts-code-highlight"><span v-for="(line, index) in groupEditorLines" :key="index" :class="{ comment: isHostsComment(line) }">{{ line || " " }}</span></pre>
                  </div>
                  <textarea
                    v-model="hostGroupContent"
                    class="hosts-native-editor"
                    spellcheck="false"
                    placeholder="127.0.0.1 example.com&#10;# 127.0.0.1 disabled.example.com"
                    aria-label="Hosts 分组内容"
                    @scroll="syncHostsEditorScroll($event, groupEditorGutter, groupEditorCode)"
                  ></textarea>
                </div>
                <div class="hosts-entry-preview">
                  <div class="hosts-preview-heading">
                    <div><strong>映射预览</strong><span>{{ selectedDnsEntries.length }} 条记录</span></div>
                    <span v-if="selectedDnsEntries.some((entry) => entry.duplicate)" class="hosts-duplicate-summary"><AlertTriangle :size="14" />发现重复记录</span>
                  </div>
                  <div v-if="selectedDnsEntries.length" class="hosts-entry-grid">
                    <div v-for="entry in selectedDnsEntries" :key="`${entry.line}-${entry.ip}`" class="hosts-entry-card" :class="{ duplicate: entry.duplicate }">
                      <div class="hosts-entry-ip"><span class="mono">{{ entry.ip }}</span><small>第 {{ entry.line }} 行</small></div>
                      <div class="hosts-entry-domains"><span v-for="domain in entry.domains" :key="domain" class="hosts-domain-chip" :class="{ duplicate: entry.duplicate }">{{ domain }}</span></div>
                      <AlertTriangle v-if="entry.duplicate" :size="16" class="hosts-duplicate-icon" />
                    </div>
                  </div>
                  <div v-else class="hosts-preview-empty">输入符合“IP 域名”的记录后，这里会显示结构化预览。</div>
                </div>
                <p class="footnote">
                  <CircleHelp :size="15" />使用原生 hosts 格式：每行“IP
                  域名”，行首添加
                  <code>#</code> 可注释；分组开关会整体启用或停用当前分组。
                </p>
              </section>
              <div v-else class="empty hosts-editor-empty">
                <Server :size="30" />
                <h3>
                  {{ desktop ? "暂无 DNS 分组" : "桌面应用中管理本地 hosts" }}
                </h3>
                <p>在左侧创建一个分组开始配置。</p>
              </div>
            </div>
          </section>
        </template>
        <template v-else-if="tab === 'cloud'">
          <section class="cloud-intro">
            <span class="cloud-intro-icon"><Cloud :size="23" /></span>
            <div>
              <strong>云资源按账号隔离</strong>
              <p>
                已支持阿里云账号的本地安全保存；腾讯云、华为云、火山引擎将在适配对应
                API 后逐项开放。
              </p>
            </div>
          </section>
          <div class="cloud-tabs" role="tablist" aria-label="云资源子菜单">
            <button
              :class="{ active: cloudSubtab === 'accounts' }"
              @click="cloudSubtab = 'accounts'"
            >
              云账号 <span>{{ cloudAccounts.length }}</span></button
            ><button
              :class="{ active: cloudSubtab === 'security-groups' }"
              @click="cloudSubtab = 'security-groups'"
            >
              安全组
            </button>
          </div>
          <template v-if="cloudSubtab === 'accounts'">
            <section class="panel cloud-accounts-panel">
              <div class="section-heading">
                <div>
                  <h3>云账号</h3>
                  <p>
                    凭据仅加密保存在 macOS
                    钥匙串；本地配置与云端资源不会跨账号混合。
                  </p>
                </div>
                <div class="toolbar">
                  <button
                    :disabled="cloudLoading || !desktop"
                    @click="loadCloudAccounts"
                  >
                    <RefreshCw
                      :size="15"
                      :class="{ spin: cloudLoading }"
                    />刷新</button
                  ><button
                    class="primary"
                    :disabled="!desktop"
                    @click="openCloudAccountDialog"
                  >
                    <Plus :size="16" />添加云账号
                  </button>
                </div>
              </div>
              <div v-if="cloudAccounts.length" class="cloud-account-list">
                <article
                  v-for="account in cloudAccounts"
                  :key="account.id"
                  class="cloud-account-row"
                >
                  <span class="cloud-provider">Ali</span>
                  <div>
                    <strong>{{ account.display_name }}</strong
                    ><small
                      >{{
                        account.auth_method === "ram_access_key"
                          ? "RAM 子账户凭据"
                          : "AccessKey"
                      }}
                      · {{ account.access_key_hint }}</small
                    >
                  </div>
                  <div>
                    <span class="cloud-field-label">云厂商</span
                    ><strong>阿里云</strong>
                  </div>
                  <div>
                    <span class="cloud-field-label">验证状态</span
                    ><span
                      class="pill"
                      :class="{
                        green: account.verification_status === '已验证',
                      }"
                      >{{ account.verification_status }}</span
                    >
                  </div>
                  <div class="cloud-row-actions">
                    <button
                      :disabled="busy"
                      @click="verifyCloudAccount(account)"
                    >
                      验证</button
                    ><button
                      class="danger-text"
                      :disabled="busy"
                      @click="deleteCloudAccount(account)"
                    >
                      删除
                    </button>
                  </div>
                </article>
              </div>
              <div v-else class="empty cloud-empty">
                <Cloud :size="28" />
                <h3>还没有云账号</h3>
                <p>
                  添加阿里云 RAM 子账户凭据或专用 AccessKey 后，即可配置安全组。
                </p>
              </div>
              <p class="footnote">
                <CircleHelp :size="15" />账号保存前通过阿里云 STS
                验证；验证后可读取地域、安全组与规则。
              </p>
            </section>
          </template>
          <template v-else>
            <section class="panel cloud-security-panel">
              <div class="section-heading">
                <div>
                  <h3>安全组</h3>
                  <p>从已验证阿里云账号读取地域、安全组与规则。</p>
                </div>
                <button
                  :disabled="cloudGroupsLoading || !selectedCloudRegion"
                  @click="loadCloudSecurityGroups"
                >
                  <RefreshCw
                    :size="15"
                    :class="{ spin: cloudGroupsLoading }"
                  />{{ cloudGroupsLoading ? "读取中…" : "刷新安全组" }}
                </button>
              </div>
              <div class="cloud-security-filters">
                <label
                  >云账号<select
                    v-model="selectedCloudAccountId"
                    :disabled="cloudRegionLoading"
                    @change="loadCloudRegions"
                  >
                    <option value="">请选择已验证账号</option>
                    <option
                      v-for="account in cloudAccounts.filter(
                        (account) => account.verification_status === '已验证',
                      )"
                      :key="account.id"
                      :value="account.id"
                    >
                      {{ account.display_name }} ·
                      {{ account.verified_account_id }}
                    </option>
                  </select></label
                ><label
                  >地域<select
                    v-model="selectedCloudRegion"
                    :disabled="!cloudRegions.length || cloudRegionLoading"
                    @change="loadCloudSecurityGroups"
                  >
                    <option value="">
                      {{ cloudRegionLoading ? "正在读取地域…" : "请选择地域" }}
                    </option>
                    <option
                      v-for="region in cloudRegions"
                      :key="region.id"
                      :value="region.id"
                    >
                      {{ region.name }} · {{ region.id }}
                    </option>
                  </select></label
                >
              </div>
              <div
                v-if="cloudSecurityGroups.length"
                class="cloud-security-list"
              >
                <article
                  v-for="group in cloudSecurityGroups"
                  :key="group.id"
                  class="cloud-security-row"
                  :class="{ selected: selectedSecurityGroupId === group.id }"
                >
                  <span class="cloud-provider">SG</span>
                  <div>
                    <strong>{{ group.name || "未命名安全组" }}</strong
                    ><small class="mono">{{ group.id }}</small>
                  </div>
                  <div>
                    <span class="cloud-field-label">VPC</span
                    ><strong class="mono">{{
                      group.vpc_id || "默认网络"
                    }}</strong>
                  </div>
                  <div>
                    <span class="cloud-field-label">类型</span
                    ><span class="pill">{{
                      group.group_type || "普通安全组"
                    }}</span>
                  </div>
                  <button @click="selectCloudSecurityGroup(group)">
                    {{
                      selectedSecurityGroupId === group.id
                        ? "已选择"
                        : "查看规则"
                    }}
                  </button>
                </article>
              </div>
              <div v-else class="empty cloud-empty">
                <ShieldCheck :size="28" />
                <h3>
                  {{
                    selectedCloudRegion
                      ? "暂无安全组"
                      : "选择账号与地域后读取安全组"
                  }}
                </h3>
                <p>
                  {{
                    selectedCloudRegion
                      ? "该地域未返回可访问的安全组。"
                      : "需要 ECS 只读权限 ecs:DescribeRegions 与 ecs:DescribeSecurityGroups。"
                  }}
                </p>
              </div>
            </section>
            <section
              v-if="selectedSecurityGroupId"
              class="panel cloud-rules-panel"
            >
              <div class="section-heading">
                <div>
                  <h3>
                    安全组规则
                    <span class="count">{{ cloudSecurityRules.length }}</span>
                  </h3>
                  <p class="mono">{{ selectedSecurityGroupId }}</p>
                </div>
                <div class="toolbar">
                  <button
                    :disabled="cloudRulesLoading"
                    @click="loadCloudSecurityRules"
                  >
                    <RefreshCw
                      :size="15"
                      :class="{ spin: cloudRulesLoading }"
                    />刷新规则</button
                  ><button class="primary" @click="openManagedRuleDialog">
                    <Plus :size="15" />添加受管规则
                  </button>
                </div>
              </div>
              <div class="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th>方向</th>
                      <th>协议 / 端口</th>
                      <th>授权对象</th>
                      <th>优先级</th>
                      <th>描述</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
                      v-for="rule in cloudSecurityRules"
                      :key="rule.id || `${rule.direction}-${rule.description}`"
                    >
                      <td>
                        <span
                          class="pill"
                          :class="{ green: rule.direction === 'ingress' }"
                          >{{
                            rule.direction === "ingress" ? "入方向" : "出方向"
                          }}</span
                        >
                      </td>
                      <td class="mono">
                        {{ rule.protocol }} · {{ rule.port_range }}
                      </td>
                      <td class="mono">{{ rule.source_cidr || "—" }}</td>
                      <td>{{ rule.priority }}</td>
                      <td>
                        <span
                          :class="{ 'managed-description': rule.managed }"
                          >{{ rule.description || "—" }}</span
                        ><small v-if="rule.managed" class="cell-sub"
                          >DomainEgress 受管</small
                        >
                      </td>
                    </tr>
                  </tbody>
                </table>
                <div v-if="!cloudSecurityRules.length" class="empty">
                  {{
                    cloudRulesLoading
                      ? "正在读取规则…"
                      : "没有可显示的安全组规则"
                  }}
                </div>
              </div>
            </section>
            <section
              v-if="managedRules.length"
              class="panel cloud-managed-panel"
            >
              <div class="section-heading">
                <div>
                  <h3>受管出口 IP 规则</h3>
                  <p>
                    每 5 分钟多源探测当前公网 IPv4；仅更新带固定唯一标识的规则。
                  </p>
                </div>
                <button
                  :disabled="busy || !desktop"
                  @click="syncAllManagedRules(false)"
                >
                  立即同步全部
                </button>
              </div>
              <div class="cloud-managed-list">
                <article v-for="rule in managedRules" :key="rule.id">
                  <div>
                    <strong class="mono">{{ rule.description }}</strong
                    ><small
                      >{{ rule.protocol }} {{ rule.port_range }} ·
                      {{ rule.region }} · {{ rule.security_group_id }}</small
                    >
                  </div>
                  <div>
                    <span class="cloud-field-label">当前授权</span
                    ><strong class="mono">{{
                      rule.last_source_cidr || "尚未同步"
                    }}</strong>
                  </div>
                  <div>
                    <span class="cloud-field-label">状态</span
                    ><span
                      class="pill"
                      :class="
                        rule.last_error
                          ? 'red'
                          : rule.last_synced_at
                            ? 'green'
                            : ''
                      "
                      >{{
                        rule.last_error
                          ? "需处理"
                          : rule.last_synced_at
                            ? "已同步"
                            : "尚未同步"
                      }}</span
                    >
                  </div>
                  <div class="cloud-row-actions">
                    <button
                      :disabled="busy || !desktop"
                      @click="syncManagedRule(rule.id)"
                    >
                      同步当前 IP</button
                    ><button
                      class="danger-text"
                      :disabled="busy || !desktop"
                      @click="deleteManagedRule(rule)"
                    >
                      删除
                    </button>
                  </div>
                  <p v-if="rule.last_error" class="cloud-managed-error">
                    {{ rule.last_error }}
                  </p>
                </article>
              </div>
            </section>
          </template>
        </template>
        <div
          v-if="managedRuleDialog"
          class="modal-backdrop"
          @click.self="managedRuleDialog = false"
        >
          <section
            class="modal cloud-account-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="managed-rule-title"
          >
            <div class="section-heading">
              <div>
                <h2 id="managed-rule-title">添加受管入方向规则</h2>
                <p>授权对象由多个 HTTP 与 DNS 探测源自动获取当前公网 IPv4。</p>
              </div>
              <button aria-label="关闭" @click="managedRuleDialog = false">
                <X :size="16" />
              </button>
            </div>
            <div class="cloud-managed-target">
              <span>安全组</span
              ><strong class="mono">{{
                managedRuleForm.security_group_id
              }}</strong>
            </div>
            <label
              >授权对象
              <div class="cloud-source-field">
                <input
                  class="mono"
                  :value="managedRuleSourceCidr"
                  readonly
                  :placeholder="
                    managedRuleSourceLoading
                      ? '正在多源探测当前公网 IPv4…'
                      : '尚未获取'
                  "
                /><button
                  type="button"
                  :disabled="managedRuleSourceLoading"
                  @click="refreshManagedRuleSource"
                >
                  <RefreshCw
                    :size="15"
                    :class="{ spin: managedRuleSourceLoading }"
                  />重新探测
                </button>
              </div>
              <small v-if="managedRuleSourceError" class="cloud-source-error">{{
                managedRuleSourceError
              }}</small
              ><small v-else
                >不可用来源会自动切换；提交时再次探测，多个成功结果必须一致。</small
              ></label
            ><label
              >协议<select
                v-model="managedRuleForm.protocol"
                @change="
                  managedRuleForm.port_range = ['ICMP', 'GRE', 'ALL'].includes(
                    managedRuleForm.protocol,
                  )
                    ? '-1/-1'
                    : '22/22'
                "
              >
                <option>TCP</option>
                <option>UDP</option>
                <option>ICMP</option>
                <option>GRE</option>
                <option>ALL</option>
              </select></label
            ><label
              >端口范围<input
                v-model="managedRuleForm.port_range"
                :disabled="
                  ['ICMP', 'GRE', 'ALL'].includes(managedRuleForm.protocol)
                "
                placeholder="22/22" /></label
            ><label
              >优先级<input
                v-model.number="managedRuleForm.priority"
                type="number"
                min="1"
                max="100"
            /></label>
            <p class="cloud-modal-hint">
              <ShieldCheck :size="15" />描述自动固定为 DomainEgress:v1:&lt;唯一
              ID&gt;。后续只按该完整标识更新，不会覆盖其他规则。
            </p>
            <div class="toolbar">
              <button :disabled="busy" @click="managedRuleDialog = false">
                取消</button
              ><button
                class="primary"
                :disabled="
                  busy ||
                  !desktop ||
                  managedRuleSourceLoading ||
                  !managedRuleSourceCidr
                "
                @click="createManagedRule"
              >
                确认并写入阿里云
              </button>
            </div>
          </section>
        </div>
        <div
          v-if="cloudAccountDialog"
          class="modal-backdrop"
          @click.self="cloudAccountDialog = false"
        >
          <section
            class="modal cloud-account-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="cloud-account-title"
          >
            <div class="section-heading">
              <div>
                <h2 id="cloud-account-title">添加云账号</h2>
                <p>当前仅支持阿里云；验证成功后凭据才会写入 macOS 钥匙串。</p>
              </div>
              <button aria-label="关闭" @click="cloudAccountDialog = false">
                <X :size="16" />
              </button>
            </div>
            <label
              >账号备注<input
                v-model="cloudAccountForm.display_name"
                placeholder="例如：生产 RAM 子账户"
                autocomplete="off" /></label
            ><label
              >认证方式<select v-model="cloudAccountForm.auth_method">
                <option value="ram_access_key">RAM 子账户凭据</option>
                <option value="access_key">专用 AccessKey</option>
              </select></label
            ><label
              >AccessKey ID<input
                v-model="cloudAccountForm.access_key_id"
                placeholder="LTAI…"
                autocomplete="off" /></label
            ><label
              >AccessKey Secret<input
                v-model="cloudAccountForm.access_key_secret"
                type="password"
                placeholder="仅用于身份验证与加密保存"
                autocomplete="new-password"
            /></label>
            <p class="cloud-modal-hint">
              <CircleHelp :size="15" />不支持控制台密码登录。保存时会向阿里云
              STS 发送签名请求验证账号身份。
            </p>
            <div class="toolbar">
              <button :disabled="busy" @click="cloudAccountDialog = false">
                取消</button
              ><button
                class="primary"
                :disabled="busy"
                @click="saveCloudAccount"
              >
                验证并保存
              </button>
            </div>
          </section>
        </div>
        <template v-if="tab === 'overview'">
          <section class="status-card">
            <div class="status-icon" :class="{ stopped: !running }">
              <ShieldCheck :size="32" />
            </div>
            <div class="status-text">
              <span class="section-label">代理服务</span>
              <h2>
                {{ running ? "连接已就绪" : "准备好安全连接"
                }}<span class="pill" :class="{ green: running }">{{
                  running ? "运行中" : "已停止"
                }}</span>
              </h2>
              <p>
                {{
                  running
                    ? "正在根据访问策略处理本机代理请求"
                    : "启动代理，为网络访问应用你的准出策略"
                }}
              </p>
              <div class="service-endpoints">
                <span
                  >HTTP/HTTPS {{ saved.http_host }}:{{ saved.http_port }}</span
                ><span
                  >SOCKS5 {{ saved.socks_host }}:{{ saved.socks_port }}</span
                >
              </div>
            </div>
            <div class="status-actions">
              <button
                :class="running ? 'secondary' : 'primary'"
                :disabled="busy || !desktop || !connected"
                @click="toggle"
              >
                <CircleStop v-if="running" :size="15" /><Play
                  v-else
                  :size="15"
                />{{ running ? "停止代理" : "启动代理" }}</button
              ><label class="overview-auto-start"
                ><input
                  v-model="config.auto_start"
                  type="checkbox"
                  role="switch"
                  aria-label="随应用自动启动代理"
                /><span
                  ><strong>随应用自动启动代理</strong
                  ><small>{{
                    config.auto_start ? "已开启，保存配置后生效" : "默认关闭"
                  }}</small></span
                ></label
              >
            </div>
          </section>
          <section class="panel network-summary">
            <div class="section-heading">
              <div>
                <h3>网络地址</h3>
                <p>本机网卡地址与系统外网出口 IP。</p>
              </div>
              <Network :size="21" class="muted" />
            </div>
            <div class="network-summary-grid">
              <div class="network-block">
                <h4>本机网卡地址</h4>
                <p>当前电脑有线与无线网卡的内网地址。</p>
                <div v-if="interfaces.length" class="interface-list">
                  <div
                    v-for="item in interfaces"
                    :key="item.name"
                    class="interface-row"
                  >
                    <span class="interface-kind">{{
                      item.kind || item.name
                    }}</span
                    ><span class="interface-name mono">{{ item.name }}</span
                    ><span class="interface-addresses"
                      ><span
                        v-for="address in item.addresses"
                        :key="address"
                        class="interface-address mono"
                        ><span class="address-family">{{
                          address.includes(":") ? "IPv6" : "IPv4"
                        }}</span
                        >{{ address }}</span
                      ></span
                    >
                  </div>
                </div>
                <div v-else class="empty">暂未检测到本机网卡地址</div>
              </div>
              <div class="network-block public-ip-block">
                <div class="network-block-heading">
                  <div>
                    <h4>外网出口 IP</h4>
                    <p>
                      HTTP 与 DNS/TCP
                      分别探测；相同只显示一个地址，不同则并列显示。
                    </p>
                  </div>
                  <button
                    :disabled="publicIpBusy || !desktop"
                    @click="probePublicIp"
                  >
                    <RefreshCw
                      :size="15"
                      :class="['refresh-icon', { 'is-spinning': publicIpBusy }]"
                    />{{ publicIpBusy ? "探测中…" : "双源探测" }}
                  </button>
                </div>
                <div
                  v-for="egress in egressProbes"
                  :key="egress.label"
                  class="public-ip-result"
                >
                  <div>
                    <span class="pill">{{ egress.label }}</span
                    ><span
                      v-if="!egress.value.addresses.length"
                      class="muted"
                      >{{ egress.value.error || "尚未探测" }}</span
                    >
                  </div>
                  <span
                    class="pill"
                    :class="{
                      green: egress.value.confidence === 'HTTP 与 DNS 一致',
                      red:
                        egress.value.confidence === 'HTTP 与 DNS 不一致' ||
                        egress.value.confidence === '探测失败',
                    }"
                    >{{ egress.value.confidence }}</span
                  >
                  <div
                    v-for="address in egress.value.addresses"
                    :key="address.source"
                    class="probe-address"
                  >
                    <div>
                      <strong class="mono">{{ address.ip }}</strong
                      ><span class="location-meta">{{ address.source }}</span>
                    </div>
                    <template v-if="address.location"
                      ><span class="location-value">{{
                        locationLabel(address.location)
                      }}</span
                      ><span class="location-meta"
                        >{{ address.location.confidence }} ·
                        {{ address.location.source
                        }}<template v-if="address.location.isp">
                          · {{ address.location.isp }}</template
                        ></span
                      ></template
                    ><span v-else class="location-meta">归属地暂不可用</span>
                  </div>
                  <span
                    v-if="egress.value.error && egress.value.addresses.length"
                    class="location-meta"
                    >{{ egress.value.error }}</span
                  >
                </div>
                <p class="footnote">
                  DNS 使用 TCP 连接
                  OpenDNS（208.67.222.222）；在线归属地查询会将出口 IP
                  发送至第三方服务。
                </p>
              </div>
            </div>
          </section>
          <div class="metrics">
            <section class="metric">
              <span>当前访问策略<ShieldCheck :size="17" /></span
              ><strong
                >{{ saved.access_mode === "whitelist" ? "白名单" : "黑名单"
                }}<small>模式</small></strong
              >
              <p>
                {{ saved[saved.access_mode].length }} 条已保存规则<button
                  @click="navigate('rules')"
                >
                  管理规则 <ArrowUpRight :size="14" />
                </button>
              </p>
            </section>
            <section class="metric">
              <span>{{ trendRangeLabel }}放行<ArrowUpRight :size="17" /></span
              ><strong>{{ admitted.toLocaleString() }}<small>次</small></strong>
              <p>按当前趋势范围统计</p>
            </section>
            <section class="metric">
              <span>已记录的拦截<ShieldCheck :size="17" /></span
              ><strong>{{ blocked.toLocaleString() }}<small>次</small></strong>
              <p>当前内存日志中的拦截记录</p>
            </section>
          </div>
          <section class="panel">
            <div class="section-heading">
              <div>
                <h3>访问趋势</h3>
                <p>{{ trendRangeLabel }} · 策略放行次数</p>
              </div>
              <div class="toolbar">
                <select v-model.number="trendRange" aria-label="趋势范围">
                  <option :value="10">10 分钟</option>
                  <option :value="30">30 分钟</option>
                  <option :value="60">1 小时</option>
                  <option :value="180">3 小时</option>
                  <option :value="360">6 小时</option>
                  <option :value="720">12 小时</option>
                  <option :value="1440">24 小时</option></select
                ><select
                  v-model.number="trendInterval"
                  aria-label="趋势统计间隔"
                >
                  <option :value="30">30 秒</option>
                  <option :value="60">1 分钟</option>
                  <option :value="300">5 分钟</option>
                  <option :value="600">10 分钟</option>
                  <option :value="1800">30 分钟</option></select
                ><span class="legend"
                  ><span class="dot live"></span>已放行请求</span
                >
              </div>
            </div>
            <div class="chart-scroll">
              <svg
                class="line-chart"
                :width="Math.max(720, buckets.length * 54)"
                height="220"
                role="img"
                :aria-label="trendRangeLabel + '放行 ' + admitted + ' 次'"
              >
                <polyline
                  :points="
                    buckets
                      .map(
                        (b, i) =>
                          i * 54 + 30 + ',' + (190 - (b.count / maximum) * 160),
                      )
                      .join(' ')
                  "
                  fill="none"
                  stroke="var(--accent)"
                  stroke-width="3"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
                <circle
                  v-for="(bucket, i) in buckets"
                  :key="bucket.label + i"
                  :cx="i * 54 + 30"
                  :cy="190 - (bucket.count / maximum) * 160"
                  r="4"
                  fill="var(--accent)"
                />
                <text
                  v-for="(bucket, i) in buckets"
                  :key="'label-' + bucket.label + i"
                  :x="i * 54 + 30"
                  y="214"
                  text-anchor="middle"
                >
                  {{ bucket.label }}
                </text>
              </svg>
            </div>
          </section>
          <div class="endpoints">
            <section class="endpoint">
              <span class="endpoint-icon"><Globe2 :size="22" /></span>
              <div>
                <h3>HTTP / HTTPS</h3>
                <code>{{ saved.http_host }}:{{ saved.http_port }}</code>
              </div>
              <span class="pill">CONNECT</span>
            </section>
            <section class="endpoint">
              <span class="endpoint-icon"><Network :size="22" /></span>
              <div>
                <h3>SOCKS5</h3>
                <code>{{ saved.socks_host }}:{{ saved.socks_port }}</code>
              </div>
              <span class="pill">TCP</span>
            </section>
          </div>
          <div class="footnote">
            <CircleHelp
              :size="15"
            />请在浏览器或其他应用中手动配置上述代理地址。
          </div>
        </template>

        <template v-if="tab === 'rules'">
          <section class="panel">
            <div class="section-heading">
              <div>
                <h3>准出策略</h3>
                <p>白名单仅允许匹配的地址；黑名单拒绝匹配的地址。</p>
              </div>
              <div class="segmented">
                <button
                  :class="{ selected: config.access_mode === 'whitelist' }"
                  @click="config.access_mode = 'whitelist'"
                >
                  白名单</button
                ><button
                  :class="{ selected: config.access_mode === 'blacklist' }"
                  @click="config.access_mode = 'blacklist'"
                >
                  黑名单
                </button>
              </div>
            </div>
            <div class="rule-hint">
              <ShieldCheck :size="18" />{{
                config.access_mode === "whitelist"
                  ? "白名单为空时，拒绝全部访问。"
                  : "黑名单为空时，允许全部访问。"
              }}
              精确域名不会自动匹配子域名。
            </div>
            <label class="field-label" for="rules-input">添加域名或 IP</label
            ><textarea
              id="rules-input"
              v-model="draft"
              rows="3"
              placeholder="example.com&#10;*.example.com&#10;每行一条，或使用逗号分隔"
            ></textarea>
            <div class="toolbar">
              <span class="muted">*.example.com 只匹配一级子域名</span
              ><button
                class="primary"
                :disabled="!draft.trim()"
                @click="addRules"
              >
                <Plus :size="16" />添加规则
              </button>
            </div>
          </section>
          <section class="panel">
            <div class="section-heading">
              <h3>
                {{ modeName }}
                <span class="count"
                  >{{ orderedRules.length }} / {{ activeRules.length }}</span
                >
              </h3>
              <div class="toolbar">
                <div class="search rule-search">
                  <Search :size="16" /><input
                    v-model="ruleSearch"
                    aria-label="搜索规则"
                    placeholder="模糊搜索规则"
                  />
                </div>
                <select v-model="ruleSort" aria-label="规则排序">
                  <option value="name">按名称排序</option>
                  <option value="time">按添加时间</option></select
                ><button
                  aria-label="复制规则"
                  title="复制规则"
                  @click="copyRules"
                >
                  <Copy :size="17" /></button
                ><button
                  class="danger-text"
                  :disabled="!activeRules.length"
                  @click="confirmAction = 'clear'"
                >
                  清空
                </button>
              </div>
            </div>
            <div class="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>域名 / IP 地址</th>
                    <th>添加时间</th>
                    <th class="right">操作</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="rule in orderedRules" :key="rule">
                    <td class="mono">
                      <Globe2 :size="15" class="inline-icon" />{{ rule }}
                    </td>
                    <td class="muted">{{ ruleDate(timestamps[rule] || 0) }}</td>
                    <td class="right">
                      <button
                        :aria-label="`删除 ${rule}`"
                        class="danger-text"
                        @click="removeRule(rule)"
                      >
                        <Trash2 :size="16" />
                      </button>
                    </td>
                  </tr>
                </tbody>
              </table>
              <div v-if="!orderedRules.length" class="empty">
                {{
                  activeRules.length
                    ? "没有匹配的规则。"
                    : `当前${modeName}为空，请添加规则。`
                }}
              </div>
            </div>
          </section>
        </template>

        <template v-if="tab === 'logs'">
          <section class="panel">
            <div class="section-heading">
              <div>
                <div class="search">
                  <Search :size="17" /><input
                    v-model="search"
                    aria-label="搜索日志"
                    placeholder="搜索来源、方法、目标或参数"
                  />
                </div>
                <div class="toolbar log-controls">
                  <select v-model="logLevel" aria-label="日志级别">
                    <option value="all">全部级别</option>
                    <option value="error">ERROR</option>
                    <option value="warn">WARN</option>
                    <option value="info">INFO</option>
                    <option value="debug">DEBUG</option></select
                  ><select v-model="logOutcome" aria-label="结果">
                    <option value="all">全部结果</option>
                    <option value="放行">放行</option>
                    <option value="拦截">拦截</option></select
                  ><select
                    v-model.number="trendRange"
                    aria-label="拦截统计时间"
                  >
                    <option :value="10">最近 10 分钟</option>
                    <option :value="30">最近 30 分钟</option>
                    <option :value="60">最近 1 小时</option>
                    <option :value="180">最近 3 小时</option>
                    <option :value="360">最近 6 小时</option>
                    <option :value="720">最近 12 小时</option>
                    <option :value="1440">最近 24 小时</option></select
                  ><button
                    :class="{ selected: autoScrollLogs }"
                    @click="autoScrollLogs = !autoScrollLogs"
                  >
                    <ArrowUpRight :size="15" />实时滚动日志</button
                  ><button
                    class="danger-text"
                    :disabled="!logs.length || !desktop"
                    @click="clearLogs"
                  >
                    <Trash2 :size="15" />删除日志
                  </button>
                </div>
              </div>
            </div>
            <div ref="logPanel" class="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>时间 / 来源</th>
                    <th>级别</th>
                    <th>方法 / 目标</th>
                    <th>参数</th>
                    <th>结果</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(log, index) in filteredLogs" :key="index">
                    <td>
                      {{ date(log.timestamp)
                      }}<small class="cell-sub mono">{{ log.source }}</small>
                    </td>
                    <td>
                      <span class="pill">{{ log.level.toUpperCase() }}</span>
                    </td>
                    <td>
                      <strong>{{ log.method }}</strong
                      ><small
                        class="cell-sub mono"
                        @contextmenu.prevent.stop="
                          showTargetMenu($event, log.target)
                        "
                        >{{ log.target }}</small
                      >
                    </td>
                    <td class="params">{{ log.params || "—" }}</td>
                    <td>
                      <span
                        class="pill"
                        :class="log.outcome === '拦截' ? 'red' : 'green'"
                        >{{ log.outcome }}</span
                      >
                    </td>
                  </tr>
                </tbody>
              </table>
              <div v-if="!filteredLogs.length" class="empty">
                <ListFilter :size="30" />
                <h3>
                  {{ logs.length ? "没有符合条件的日志" : "暂无访问日志" }}
                </h3>
                <p>代理收到请求后，符合日志级别的记录将显示在这里。</p>
              </div>
            </div>
            <p class="footnote">
              显示 {{ filteredLogs.length }} 条 · 每 1.5 秒刷新 · 最多 2,000
              条内存日志，退出后清空。
            </p>
          </section>
          <section class="panel">
            <div class="section-heading">
              <div>
                <h3>
                  拦截域名统计
                  <span class="count">{{ blockedDomains.length }}</span>
                </h3>
                <p>按目标域名与请求来源去重 · {{ trendRangeLabel }}</p>
              </div>
            </div>
            <div class="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>拦截域名</th>
                    <th>请求来源</th>
                    <th>拦截次数</th>
                    <th>最近拦截时间</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="(log, index) in blockedDomains"
                    :key="`${log.target}-${log.source}-${index}`"
                  >
                    <td class="mono">{{ log.target }}</td>
                    <td class="mono">{{ log.source }}</td>
                    <td>{{ log.count }}</td>
                    <td class="muted">{{ date(log.timestamp) }}</td>
                  </tr>
                </tbody>
              </table>
              <div v-if="!blockedDomains.length" class="empty">
                当前时间范围内没有拦截域名。
              </div>
            </div>
          </section>
        </template>

        <template v-if="tab === 'ports'">
          <section class="panel">
            <div class="section-heading">
              <div class="search">
                <Search :size="17" /><input
                  v-model="portSearch"
                  aria-label="搜索端口"
                  placeholder="搜索端口、进程或 PID"
                />
              </div>
              <button :disabled="portBusy || !desktop" @click="loadPorts">
                <RefreshCw :size="16" :class="{ spin: portBusy }" />{{
                  portBusy ? "查询中…" : "刷新"
                }}
              </button>
            </div>
            <div class="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>监听地址</th>
                    <th>进程 / PID</th>
                    <th>启动时间 / 已运行</th>
                    <th class="right">操作</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="p in filteredPorts" :key="`${p.pid}-${p.port}`">
                    <td class="mono">{{ p.port }}</td>
                    <td>
                      {{ p.name
                      }}<small class="cell-sub mono">{{ p.pid }}</small>
                    </td>
                    <td>
                      {{ p.started
                      }}<small class="cell-sub">{{ p.elapsed }}</small>
                    </td>
                    <td class="right">
                      <button class="danger-text" @click="confirmAction = p">
                        结束进程
                      </button>
                    </td>
                  </tr>
                </tbody>
              </table>
              <div v-if="!filteredPorts.length" class="empty">
                {{
                  portBusy ? "正在读取系统监听端口…" : "没有可显示的监听端口"
                }}
              </div>
            </div>
            <p class="footnote">
              数据来自 macOS lsof，只显示当前用户可见的 TCP 监听进程；当前每
              {{ saved.port_refresh_interval_seconds }} 秒自动刷新。
            </p>
          </section>
        </template>

        <template v-if="tab === 'settings' || tab === 'ssh-forward'">
          <div v-if="tab === 'ssh-forward'" class="forward-actions">
            <button
              :disabled="busy || !config.ssh_forwards.length"
              @click="testAllSshForwards"
            >
              <Check :size="16" />批量测试全部</button
            ><span
              v-for="rule in config.ssh_forwards"
              :key="`test-${rule.id}`"
              class="forward-test-item"
              ><span>{{ rule.name }}</span
              ><button :disabled="busy" @click="testSshForward(rule)">
                测试
              </button></span
            >
          </div>
          <div
            v-if="tab === 'ssh-forward' && groupedSshForwards.length"
            class="forward-split-layout"
          >
            <aside class="forward-group-menu">
              <div class="forward-menu-title">项目分组</div>
              <button
                v-for="group in groupedSshForwards"
                :key="`menu-${group.name}`"
                class="forward-menu-item"
                :class="{ active: selectedForwardGroup === group.name }"
                @click="selectForwardGroup(group.name)"
              >
                <ChevronRight
                  :size="15"
                  :class="{ rotated: selectedForwardGroup === group.name }"
                /><span>{{ group.name }}</span
                ><small>{{ group.rules.length }}</small></button
              ><button class="forward-menu-add" @click="addSshForward">
                <Plus :size="14" />新增转发</button
              ><button
                class="forward-menu-collapse"
                :aria-label="
                  forwardMenuCollapsed ? '展开项目分组' : '收起项目分组'
                "
                :title="forwardMenuCollapsed ? '展开项目分组' : '收起项目分组'"
                @click="forwardMenuCollapsed = !forwardMenuCollapsed"
              >
                <ChevronRight
                  v-if="forwardMenuCollapsed"
                  :size="16"
                /><ChevronLeft v-else :size="16" />
              </button>
            </aside>
            <section
              class="forward-split-content"
              :class="{ collapsed: !selectedForwardGroup }"
            >
              <template v-if="selectedForwardGroup"
                ><div class="split-content-head">
                  <div>
                    <h3>{{ selectedForwardGroup }}</h3>
                    <p>{{ selectedForwardRules.length }} 条 SSH 端口转发</p>
                  </div>
                  <button :disabled="busy" @click="testAllSshForwards">
                    全部测试
                  </button>
                </div>
                <div class="split-forward-cards">
                  <article
                    v-for="rule in selectedForwardRules"
                    :key="`split-${rule.id}`"
                    class="group-forward-card"
                  >
                    <div class="ssh-forward-head">
                      <input
                        v-model="rule.project"
                        placeholder="项目分组"
                      /><input
                        v-model="rule.name"
                        placeholder="转发名称"
                      /><span
                        class="pill"
                        :class="{ green: sshForwardPorts[rule.id] }"
                        >{{
                          sshForwardPorts[rule.id]
                            ? `运行中 :${sshForwardPorts[rule.id]}`
                            : "已停止"
                        }}</span
                      ><button :disabled="busy" @click="testSshForward(rule)">
                        测试</button
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
                      <label class="local-field"
                        >本地端口<input
                          v-model.number="rule.local_port"
                          type="number"
                          min="0"
                          max="65535"
                          placeholder="自动分配" /></label
                      ><label class="local-field"
                        >本地地址<input
                          v-model="rule.bind_host"
                          placeholder="127.0.0.1" /></label
                      ><label class="remote-field"
                        >远程地址<input
                          v-model="rule.remote_host"
                          placeholder="127.0.0.1" /></label
                      ><label class="remote-field"
                        >远程端口<input
                          v-model.number="rule.remote_port"
                          type="number"
                          min="1"
                          max="65535" /></label
                      ><label class="remote-field"
                        >SSH 服务器<input
                          v-model="rule.ssh_host"
                          placeholder="~/.ssh/config 别名" /></label
                      ><label class="remote-field"
                        >SSH 端口<input
                          v-model.number="rule.ssh_port"
                          type="number"
                          min="1"
                          max="65535" /></label
                      ><label class="remote-field"
                        >用户名<input
                          v-model="rule.ssh_username"
                          placeholder="可由 SSH config 补全" /></label
                      ><label
                        >备注<input
                          v-model="rule.note"
                          placeholder="用途或环境说明"
                      /></label>
                    </div>
                    <p class="footnote">
                      {{ rule.bind_host }}:{{ rule.local_port || "自动分配" }} →
                      {{ rule.remote_host }}:{{ rule.remote_port }}
                    </p>
                  </article>
                </div></template
              >
              <div v-else class="split-empty">选择左侧项目分组查看端口转发</div>
            </section>
          </div>
          <div v-if="tab === 'ssh-forward'" class="ssh-global-options">
            <div class="options-header">
              <div>
                <strong>SSH 转发全局 -o 参数</strong
                ><small>默认参数会应用到全部端口转发。</small>
              </div>
              <button @click="showGlobalOptions = !showGlobalOptions">
                {{
                  showGlobalOptions
                    ? "收起"
                    : `展开${config.ssh_forward_options.length ? `（${config.ssh_forward_options.length}）` : ""}`
                }}
              </button>
            </div>
            <div v-if="showGlobalOptions" class="options-editor">
              <div
                v-for="(option, index) in config.ssh_forward_options"
                :key="index"
                class="option-row"
              >
                <input
                  :value="optionParts(option).key"
                  placeholder="参数名"
                  @input="
                    updateGlobalOption(
                      index,
                      ($event.target as HTMLInputElement).value,
                      optionParts(option).value,
                    )
                  "
                /><span>=</span
                ><input
                  :value="optionParts(option).value"
                  placeholder="参数值"
                  @input="
                    updateGlobalOption(
                      index,
                      optionParts(option).key,
                      ($event.target as HTMLInputElement).value,
                    )
                  "
                /><button
                  class="danger-text"
                  @click="removeGlobalOption(index)"
                >
                  删除
                </button>
              </div>
              <button @click="addGlobalOption">
                <Plus :size="14" />新增参数
              </button>
            </div>
          </div>
          <div
            v-if="tab === 'ssh-forward' && config.ssh_forwards.length"
            class="ssh-rule-options"
          >
            <div
              v-for="rule in config.ssh_forwards"
              :key="`options-${rule.id}`"
              class="rule-options-item"
            >
              <div class="options-header">
                <div>
                  <strong>{{ rule.name }} 的单条 -o 参数</strong
                  ><small>{{
                    rule.ssh_options.length
                      ? `${rule.ssh_options.length} 个参数`
                      : "未配置"
                  }}</small>
                </div>
                <button @click="toggleRuleOptions(rule.id)">
                  {{ expandedRuleOptions[rule.id] ? "收起" : "展开" }}
                </button>
              </div>
              <div v-if="expandedRuleOptions[rule.id]" class="options-editor">
                <div
                  class="option-row"
                  v-for="(option, index) in rule.ssh_options"
                  :key="index"
                >
                  <input
                    :value="optionParts(option).key"
                    placeholder="参数名"
                    @input="
                      updateRuleOption(
                        rule,
                        index,
                        ($event.target as HTMLInputElement).value,
                        optionParts(option).value,
                      )
                    "
                  /><span>=</span
                  ><input
                    :value="optionParts(option).value"
                    placeholder="参数值"
                    @input="
                      updateRuleOption(
                        rule,
                        index,
                        optionParts(option).key,
                        ($event.target as HTMLInputElement).value,
                      )
                    "
                  /><button
                    class="danger-text"
                    @click="removeRuleOption(rule, index)"
                  >
                    删除
                  </button>
                </div>
                <button @click="addRuleOption(rule)">
                  <Plus :size="14" />新增参数
                </button>
              </div>
            </div>
          </div>
          <div
            v-if="tab === 'ssh-forward' && groupedSshForwards.length"
            class="grouped-forward-list"
          >
            <section
              v-for="(group, groupIndex) in groupedSshForwards"
              :key="group.name"
              class="forward-group"
            >
              <header class="forward-group-header">
                <button
                  class="group-toggle"
                  @click="toggleForwardGroup(group.name, groupIndex)"
                >
                  <ChevronRight
                    :size="16"
                    :class="{
                      rotated: isForwardGroupExpanded(group.name, groupIndex),
                    }"
                  /><strong>{{ group.name }}</strong
                  ><span class="muted"
                    >{{ group.rules.length }} 条转发</span
                  ></button
                ><button @click="testAllSshForwards">全部测试</button>
              </header>
              <div
                v-if="isForwardGroupExpanded(group.name, groupIndex)"
                class="forward-group-body"
              >
                <article
                  v-for="rule in group.rules"
                  :key="`group-${rule.id}`"
                  class="group-forward-card"
                >
                  <div class="ssh-forward-head">
                    <input
                      v-model="rule.project"
                      placeholder="项目分组"
                    /><input v-model="rule.name" placeholder="转发名称" /><span
                      class="pill"
                      :class="{ green: sshForwardPorts[rule.id] }"
                      >{{
                        sshForwardPorts[rule.id]
                          ? `运行中 :${sshForwardPorts[rule.id]}`
                          : "已停止"
                      }}</span
                    ><button :disabled="busy" @click="testSshForward(rule)">
                      测试</button
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
                    <label class="local-field"
                      >本地端口<input
                        v-model.number="rule.local_port"
                        type="number"
                        min="0"
                        max="65535"
                        placeholder="自动分配" /></label
                    ><label class="local-field"
                      >本地地址<input
                        v-model="rule.bind_host"
                        placeholder="127.0.0.1" /></label
                    ><label class="remote-field"
                      >远程地址<input
                        v-model="rule.remote_host"
                        placeholder="127.0.0.1" /></label
                    ><label class="remote-field"
                      >远程端口<input
                        v-model.number="rule.remote_port"
                        type="number"
                        min="1"
                        max="65535" /></label
                    ><label class="remote-field"
                      >SSH 服务器<input
                        v-model="rule.ssh_host"
                        placeholder="~/.ssh/config 别名" /></label
                    ><label class="remote-field"
                      >SSH 端口<input
                        v-model.number="rule.ssh_port"
                        type="number"
                        min="1"
                        max="65535" /></label
                    ><label class="remote-field"
                      >用户名<input
                        v-model="rule.ssh_username"
                        placeholder="可由 SSH config 补全" /></label
                    ><label
                      >备注<input
                        v-model="rule.note"
                        placeholder="用途或环境说明"
                    /></label>
                  </div>
                  <p class="footnote">
                    {{ rule.bind_host }}:{{ rule.local_port || "自动分配" }} →
                    {{ rule.remote_host }}:{{ rule.remote_port }} ·
                    {{ rule.ssh_username || "config 用户" }}@{{ rule.ssh_host }}
                  </p>
                </article>
              </div>
            </section>
          </div>
          <section v-if="tab === 'settings'" class="panel font-scale-panel">
            <div class="section-heading">
              <div>
                <h3>文字大小</h3>
                <p>全局应用显示大小；保存配置后会同步到 iCloud。</p>
              </div>
            </div>
            <div class="font-scale-row">
              <div class="font-scale-buttons">
                <button
                  aria-label="缩小文字"
                  :disabled="config.font_scale === fontScaleChoices[0]"
                  @click="adjustFontScale(-1)"
                >
                  −</button
                ><output aria-live="polite">{{ config.font_scale }}%</output
                ><button
                  aria-label="放大文字"
                  :disabled="
                    config.font_scale ===
                    fontScaleChoices[fontScaleChoices.length - 1]
                  "
                  @click="adjustFontScale(1)"
                >
                  +
                </button>
              </div>
              <p class="footnote">
                快捷键：Control + 放大，Control - 缩小，Control 0 恢复 100%。
              </p>
            </div>
          </section>
          <section v-if="tab === 'settings'" class="panel">
            <div class="section-heading">
              <div>
                <h3>启动行为</h3>
                <p>
                  登录电脑后自动启动 DomainEgress；关闭后可随时从这里重新开启。
                </p>
              </div>
              <button
                class="primary"
                :disabled="autostartBusy || !desktop"
                @click="toggleAutostart"
              >
                {{
                  autostartBusy
                    ? "处理中…"
                    : autostartEnabled
                      ? "关闭开机启动"
                      : "开启开机启动"
                }}
              </button>
            </div>
            <p class="footnote">
              当前状态：{{
                autostartEnabled ? "已开启" : "未开启"
              }}。此设置写入当前用户的系统登录启动项。
            </p>
          </section>
          <section v-if="tab === 'settings'" class="panel appearance-panel">
            <div class="section-heading">
              <div>
                <h3>外观与主题</h3>
                <p>即时生效并自动保存到本机，无需点击“保存配置”。</p>
              </div>
            </div>
            <div class="appearance-modes" role="group" aria-label="外观模式">
              <button
                v-for="mode in ['system', 'light', 'dark'] as const"
                :key="mode"
                :aria-pressed="appearance === mode"
                :class="{ selected: appearance === mode }"
                @click="setAppearance(mode)"
              >
                {{
                  { system: "跟随系统", light: "浅色模式", dark: "暗黑模式" }[
                    mode
                  ]
                }}
              </button>
            </div>
            <div class="theme-grid" role="group" aria-label="程序员颜色主题">
              <button
                v-for="item in themes"
                :key="item.id"
                class="theme-card"
                :class="{ selected: theme === item.id }"
                :aria-pressed="theme === item.id"
                :aria-label="`使用 ${item.name} 主题`"
                @click="setTheme(item.id)"
              >
                <span
                  class="theme-preview"
                  :style="{ background: item.bg, color: item.dark }"
                  ><span>&lt;/&gt;</span
                  ><i :style="{ background: item.dark }"></i
                  ><i :style="{ background: item.light }"></i
                ></span>
                <span class="theme-name"
                  >{{ item.name
                  }}<Check v-if="theme === item.id" :size="15" /></span
                ><small>{{ item.description }}</small>
              </button>
            </div>
            <p v-if="appearanceError" class="appearance-error" role="status">
              {{ appearanceError }}
            </p>
          </section>
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
                <p>版本 0.4.1 · 本地网络安全代理</p>
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
      </div>
    </main>
    <div
      v-if="contextMenu"
      class="context-menu"
      :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
      @click.stop
    >
      <strong>添加到{{ modeName }}</strong
      ><small class="context-target">{{ contextMenu.target }}</small
      ><label
        v-for="rule in contextMenu.candidates"
        :key="rule"
        class="candidate"
        ><input
          v-model="selectedTargets"
          type="checkbox"
          :value="rule"
        /><span>{{ rule }}</span></label
      >
      <p v-if="!contextMenu.candidates.length" class="context-empty">
        候选规则均已存在
      </p>
      <button
        class="primary"
        :disabled="!selectedTargets.length || !contextMenu.candidates.length"
        @click="addTargetRules"
      >
        添加所选规则
      </button>
    </div>
    <div v-if="confirmAction" class="modal-backdrop" @click.self="closeConfirm">
      <section
        class="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirm-title"
      >
        <h2 id="confirm-title">
          {{
            confirmAction === "clear"
              ? `清空${modeName}？`
              : confirmAction === "save-mode"
                ? `切换为${modeName}模式？`
                : "结束进程？"
          }}
        </h2>
        <p>
          {{
            confirmAction === "clear"
              ? "将清空当前列表草稿，保存后才会生效。"
              : confirmAction === "save-mode"
                ? `将切换为${modeName}模式。保存后，新的准出策略将立即作用于后续连接。`
                : `将向 ${confirmAction.name}（PID ${confirmAction.pid}）发送 SIGTERM，可能中断该应用的连接。`
          }}
        </p>
        <div class="toolbar">
          <button :disabled="busy" @click="closeConfirm">取消</button
          ><button class="danger" :disabled="busy" @click="confirm">
            {{ busy ? "处理中…" : "确认" }}
          </button>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
:global(body) {
  overflow-x: hidden;
}
.content {
  padding-top: 22px;
}
.header-separator {
  width: 1px;
  height: 14px;
  background: var(--border);
  margin: 0 4px;
}
.host-info {
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--muted);
}
.sidebar-update {
  margin-top: 10px;
  padding: 5px 0;
  border: 0;
  background: transparent;
  color: var(--muted);
  font-size: 10px;
}
.sidebar-bottom .version {
  display: block;
}
.sidebar-update {
  display: flex;
  width: max-content;
  justify-content: flex-start;
}
.sidebar-update:hover {
  color: var(--accent);
  background: transparent;
}
.sidebar-update-hint {
  display: block;
  margin-top: 3px;
  color: var(--accent);
  font-size: 9px;
}
.page-heading {
  margin-bottom: 17px;
}
.page-heading-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}
.page-heading h1 {
  line-height: 1.3;
}
.page-heading p {
  margin-top: 3px;
  line-height: 1.55;
}
.context-menu {
  position: fixed;
  z-index: 30;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 7px;
  padding: 5px;
  box-shadow: 0 8px 24px var(--shadow);
}
.context-menu button {
  border: 0;
  width: 100%;
}
.context-menu {
  min-width: 280px;
  max-width: min(360px, calc(100vw - 24px));
  padding: 12px;
}
.context-menu strong,
.context-target {
  display: block;
}
.context-target {
  color: var(--muted);
  font:
    10px "SFMono-Regular",
    Consolas,
    monospace;
  margin: 6px 0 10px;
  overflow-wrap: anywhere;
}
.candidate {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 4px;
  font:
    11px "SFMono-Regular",
    Consolas,
    monospace;
  cursor: pointer;
}
.candidate input {
  accent-color: var(--accent);
}
.context-menu .primary {
  margin-top: 8px;
}
.context-empty {
  font-size: 11px;
  padding: 4px 0;
}
.ssh-profile {
  border: 1px solid var(--border);
  border-radius: 9px;
  padding: 16px;
  margin-bottom: 12px;
}
.ssh-profile-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 13px;
}
.profile-active {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--muted);
  font-size: 11px;
}
.profile-name {
  flex: 1;
  font-weight: 600;
  min-width: 120px;
}
.ssh-hop {
  display: grid;
  grid-template-columns: 28px 2fr 90px 1.5fr 1.2fr 36px;
  gap: 8px;
  align-items: center;
  margin-bottom: 9px;
}
.hop-index {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  background: var(--raised);
  border-radius: 50%;
  color: var(--muted);
  font-size: 11px;
  cursor: grab;
  user-select: none;
}
.ssh-hop:active .hop-index {
  cursor: grabbing;
}
.ssh-profile .toolbar {
  margin-top: 10px;
}
.ssh-empty {
  padding: 28px 15px;
}
.ssh-forward {
  border: 1px solid var(--border);
  border-radius: 9px;
  padding: 16px;
  margin-bottom: 12px;
}
.ssh-forward-panel > .ssh-forwards {
  display: none;
}
.ssh-forward-panel > .ssh-rule-options {
  display: none;
}
.forward-actions .forward-test-item {
  display: none;
}
.forward-menu-collapsed .forward-split-layout {
  grid-template-columns: 34px minmax(0, 1fr);
}
.forward-menu-collapsed .forward-group-menu {
  min-height: 52px;
  padding: 10px 4px;
  box-sizing: border-box;
}
.forward-menu-collapsed .forward-menu-title,
.forward-menu-collapsed .forward-menu-item,
.forward-menu-collapsed .forward-menu-add {
  display: none;
}
.grouped-forward-list {
  display: none;
}
.forward-split-layout {
  display: grid;
  grid-template-columns: 220px minmax(0, 1fr);
  gap: 12px;
}
.forward-group-menu {
  position: relative;
  padding: 10px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-muted);
  align-self: start;
}
.forward-menu-title {
  padding: 6px 9px 10px;
  color: var(--muted);
  font-size: 11px;
}
.forward-menu-item {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 10px 9px;
  border: 0;
  border-radius: 7px;
  text-align: left;
  color: var(--text);
  background: transparent;
}
.forward-menu-item:hover,
.forward-menu-item.active {
  background: color-mix(in srgb, var(--accent) 16%, var(--surface));
}
.forward-menu-item small {
  margin-left: auto;
  color: var(--muted);
}
.forward-menu-item svg {
  transition: transform 0.16s ease;
}
.forward-menu-item svg.rotated {
  transform: rotate(90deg);
}
.forward-menu-add {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  margin-top: 7px;
  justify-content: center;
}
.forward-menu-collapse {
  position: absolute;
  top: 50%;
  right: 6px;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 28px;
  padding: 0;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--muted);
  transform: translateY(-50%);
}
.forward-menu-collapse:hover {
  color: var(--text);
  background: color-mix(in srgb, var(--accent) 12%, transparent);
}
.forward-split-content {
  min-width: 0;
  padding: 15px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface);
}
.forward-split-content.collapsed {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 170px;
}
.split-content-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border);
}
.split-content-head h3 {
  margin: 0 0 4px;
}
.split-content-head p {
  margin: 0;
  color: var(--muted);
  font-size: 12px;
}
.split-forward-cards {
  padding-top: 12px;
}
.split-empty {
  color: var(--muted);
}
.forward-group {
  border: 0;
  border-radius: 8px;
  overflow: hidden;
}
.forward-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 11px 12px;
  background: var(--surface-muted);
  border: 1px solid var(--border);
  border-radius: 8px;
}
.group-toggle {
  display: flex;
  align-items: center;
  gap: 7px;
  flex: 1;
  text-align: left;
  font-size: 13px;
}
.group-toggle svg {
  transition: transform 0.16s ease;
}
.group-toggle svg.rotated {
  transform: rotate(90deg);
}
.forward-group-body {
  position: relative;
  margin: 0 0 4px 18px;
  padding: 8px 0 4px 18px;
  border-left: 1px solid var(--border);
}
.group-forward-card {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px;
  margin-bottom: 8px;
  background: var(--surface);
}
.group-forward-card:last-child {
  margin-bottom: 0;
}
.group-forward-card .ssh-forward-grid .local-field input {
  border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  background: color-mix(in srgb, var(--accent) 7%, var(--surface));
}
.group-forward-card .ssh-forward-grid .remote-field input {
  border-color: color-mix(in srgb, #d97706 55%, var(--border));
  background: color-mix(in srgb, #d97706 8%, var(--surface));
}
.forward-view > .panel:not(.ssh-forward-panel) {
  display: none;
}
.forward-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
}
.forward-test-item {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 5px 7px 5px 10px;
  border: 1px solid var(--border);
  border-radius: 7px;
  color: var(--muted);
  font-size: 11px;
}
.ssh-global-options,
.ssh-rule-options {
  padding: 12px 14px;
  margin-bottom: 12px;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--surface-muted);
}
.options-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.options-header div {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.options-header small,
.ssh-rule-options label {
  color: var(--muted);
  font-size: 11px;
}
.options-editor {
  display: flex;
  flex-direction: column;
  gap: 9px;
  margin-top: 12px;
}
.option-row {
  display: grid;
  grid-template-columns: minmax(100px, 0.7fr) auto minmax(140px, 1.5fr) auto;
  align-items: center;
  gap: 8px;
}
.ssh-global-options label,
.ssh-rule-options label {
  display: flex;
  flex-direction: column;
  gap: 7px;
  color: var(--muted);
  font-size: 11px;
}
.ssh-global-options textarea,
.ssh-rule-options textarea {
  width: 100%;
  resize: vertical;
  font-family: var(--mono);
  line-height: 1.5;
}
.ssh-global-options small {
  display: block;
  margin-top: 7px;
  color: var(--muted);
}
.ssh-rule-options > .rule-options-item + .rule-options-item {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--border);
}
.ssh-forward-grid label:nth-child(-n + 2) input {
  border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  background: color-mix(in srgb, var(--accent) 7%, var(--surface));
}
.ssh-forward-grid label:nth-child(n + 3):nth-child(-n + 7) input {
  border-color: color-mix(in srgb, #d97706 55%, var(--border));
  background: color-mix(in srgb, #d97706 8%, var(--surface));
}
.ssh-command-parser {
  padding: 14px;
  margin: 0 0 14px;
  border: 1px dashed var(--border);
  border-radius: 9px;
  background: var(--surface-muted);
}
.parser-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.ssh-command-parser label {
  display: flex;
  flex-direction: column;
  gap: 7px;
  color: var(--muted);
  font-size: 11px;
}
.ssh-command-parser textarea {
  width: 100%;
  resize: vertical;
  font-family: var(--mono);
  line-height: 1.5;
}
.ssh-command-parser .toolbar {
  margin-top: 14px;
  gap: 12px;
}
.ssh-command-parser .toolbar .muted {
  line-height: 1.5;
}
.ssh-forward-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 13px;
}
.ssh-forward-head input:first-child {
  width: 140px;
}
.ssh-forward-head input:nth-child(2) {
  flex: 1;
  min-width: 120px;
}
.ssh-forward-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 12px;
}
.ssh-forward-grid label {
  display: flex;
  flex-direction: column;
  gap: 7px;
  color: var(--muted);
  font-size: 11px;
}
.compact-setting {
  padding: 13px 0;
  margin: 4px 0 0;
}
@media (max-width: 700px) {
  .ssh-profile-head {
    flex-wrap: wrap;
  }
  .profile-name {
    order: 2;
    flex-basis: calc(100% - 80px);
  }
  .ssh-hop {
    grid-template-columns: 28px 1fr 80px;
  }
  .ssh-hop input:nth-of-type(2),
  .ssh-hop select {
    grid-column: 2 / 4;
  }
  .ssh-hop button {
    grid-column: 3;
    grid-row: 1;
  }
}
@media (max-width: 700px) {
  .ssh-forward-head {
    flex-wrap: wrap;
  }
  .ssh-forward-head input:first-child,
  .ssh-forward-head input:nth-child(2) {
    width: 100%;
    flex-basis: 100%;
  }
  .ssh-forward-grid {
    grid-template-columns: 1fr 1fr;
  }
}
@media (max-width: 800px) {
  .forward-split-layout {
    grid-template-columns: 1fr;
  }
  .forward-group-menu {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  .forward-menu-title {
    width: 100%;
  }
  .forward-menu-item {
    width: auto;
    flex: 1 1 150px;
  }
  .forward-menu-add {
    width: auto;
  }
}
.chart-scroll {
  width: 100%;
  overflow-x: auto;
  overflow-y: hidden;
}
.line-chart {
  display: block;
  min-width: 720px;
}
.line-chart text {
  fill: var(--muted);
  font-size: 10px;
}
.service-endpoints {
  display: flex;
  gap: 14px;
  margin-top: 10px;
  color: var(--muted);
  font:
    10px "SFMono-Regular",
    Consolas,
    monospace;
}
.interface-list {
  display: grid;
  gap: 8px;
}
.interface-row {
  display: grid;
  grid-template-columns: minmax(120px, 1fr) 64px minmax(0, 2fr);
  gap: 12px;
  align-items: center;
  padding: 10px 12px;
  background: var(--raised);
  border-radius: 7px;
}
.interface-kind {
  font-size: 12px;
  font-weight: 600;
}
.interface-name {
  color: var(--muted);
  font-size: 11px;
}
.interface-addresses {
  min-width: 0;
  color: var(--text);
  font-size: 11px;
  overflow-wrap: anywhere;
}
.interface-addresses {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.interface-address {
  display: flex;
  align-items: baseline;
  gap: 9px;
  cursor: copy;
}
.address-family {
  width: 34px;
  flex: none;
  color: var(--muted);
  font:
    10px Inter,
    -apple-system,
    sans-serif;
}
.status-card {
  padding: 18px 22px;
  gap: 14px;
}
.status-icon {
  width: 52px;
  height: 52px;
  border-radius: 14px;
}
.status-text h2 {
  margin: 3px 0;
}
.service-endpoints {
  margin-top: 6px;
}
.status-actions {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 9px;
  min-width: 196px;
}
.status-actions > button {
  justify-content: center;
}
.overview-auto-start {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 9px;
  padding: 8px 10px;
  border: 1px solid color-mix(in srgb, var(--accent) 32%, var(--border));
  border-radius: 8px;
  color: var(--text);
  cursor: pointer;
}
.overview-auto-start > span {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 11px;
}
.overview-auto-start small {
  color: var(--muted);
  font-size: 10px;
  font-weight: 400;
}
.network-summary {
  padding: 19px 22px;
}
.network-summary .section-heading {
  margin-bottom: 16px;
}
.network-summary-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 14px;
}
.network-block {
  min-width: 0;
  padding: 16px;
  background: var(--surface-muted);
  border: 1px solid var(--border);
  border-radius: 10px;
}
.network-block h4 {
  margin: 0;
  font-size: 12px;
}
.network-block > p {
  margin-top: 4px;
  font-size: 10px;
}
.network-block .interface-list {
  margin-top: 11px;
}
.network-block .empty {
  padding: 25px 10px;
}
.network-block-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.network-block-heading p {
  margin-top: 4px;
  font-size: 10px;
}
.public-ip-block {
  border-left: 1px solid var(--border);
}
.public-ip-result {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 7px 12px;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid var(--border);
}
.public-ip-result:last-of-type {
  border-bottom: 0;
}
.public-ip-result > div {
  display: flex;
  align-items: center;
  gap: 9px;
  min-width: 0;
}
.probe-address {
  grid-column: 1 / -1;
  display: grid !important;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 4px 12px;
  padding: 9px 10px;
  background: var(--raised);
  border-radius: 7px;
}
.public-ip-result:has(.probe-address + .probe-address) {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}
.public-ip-result:has(.probe-address + .probe-address)
  > :not(.probe-address):not(.location-meta) {
  grid-column: 1 / -1;
}
.public-ip-result:has(.probe-address + .probe-address) .probe-address {
  grid-column: auto;
}
.probe-address > div {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 3px;
}
.probe-address .location-value {
  align-self: center;
}
.location-value {
  font-size: 12px;
  color: var(--text);
}
.location-meta {
  grid-column: 1 / -1;
  font-size: 10px;
  color: var(--muted);
  overflow-wrap: anywhere;
}
.public-ip-block .footnote {
  margin-top: 12px;
}
.refresh-icon {
  display: block;
  transform-origin: center;
}
.refresh-icon.is-spinning {
  animation: refresh-icon-spin 0.8s linear infinite;
}
@keyframes refresh-icon-spin {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .refresh-icon.is-spinning {
    animation: none;
  }
}
.update-banner {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 18px;
  padding: 12px 15px;
  color: var(--text);
  background: color-mix(in srgb, var(--accent) 12%, var(--surface));
  border: 1px solid color-mix(in srgb, var(--accent) 35%, var(--border));
  border-radius: 9px;
}
.update-banner span {
  flex: 1;
  font-size: 12px;
}
.update-dismiss {
  border: 0;
  background: transparent;
  color: var(--muted);
}
.about-version {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}
.about-version p {
  margin: 0;
}
.about-version button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.update-hint {
  color: var(--accent);
}
.public-ip-result strong {
  font-size: 18px;
  letter-spacing: 0.02em;
}
.app-shell {
  font-size: calc(13px * var(--font-scale, 1));
}
.app-shell h1 {
  font-size: calc(27px * var(--font-scale, 1));
}
.app-shell h2 {
  font-size: calc(20px * var(--font-scale, 1));
}
.app-shell h3 {
  font-size: calc(14px * var(--font-scale, 1));
}
.app-shell h4 {
  font-size: calc(12px * var(--font-scale, 1));
}
.app-shell :is(button, input, select, textarea) {
  font-size: inherit;
}
.font-scale-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
}
.font-scale-row .footnote {
  margin: 0;
}
.font-scale-buttons {
  display: flex;
  align-items: center;
  gap: 8px;
}
.font-scale-buttons button {
  width: 34px;
  height: 34px;
  padding: 0;
  font-size: 20px;
  line-height: 1;
}
.font-scale-buttons output {
  min-width: 62px;
  text-align: center;
  font: 600 15px var(--mono);
}
.notice .undo {
  color: var(--accent);
  font-weight: 600;
  padding: 4px 8px;
  background: none;
  border: 0;
}
.gist-sync-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  column-gap: 22px;
  row-gap: 18px;
}
.gist-sync-actions {
  justify-content: flex-start;
  flex-wrap: wrap;
  margin-top: 18px;
}
.gist-sync-actions .muted {
  margin-left: 4px;
}
@media (max-width: 700px) {
  .service-endpoints {
    flex-direction: column;
    gap: 4px;
  }
  .status-text {
    min-width: 0;
  }
  .service-endpoints span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
}
@media (max-width: 700px) {
  .gist-sync-grid {
    grid-template-columns: 1fr;
  }
  .gist-sync-actions {
    align-items: stretch;
  }
  .gist-sync-actions button,
  .gist-sync-actions .muted {
    width: 100%;
  }
}
@media (max-width: 700px) {
  .interface-row {
    grid-template-columns: 1fr auto;
    gap: 5px 10px;
  }
  .interface-addresses {
    grid-column: 1 / -1;
  }
}
@media (max-width: 700px) {
  .status-card {
    padding: 16px;
  }
  .network-summary {
    padding: 17px;
  }
  .network-summary-grid {
    gap: 12px;
  }
  .network-block {
    padding: 14px;
  }
  .public-ip-block {
    border-left: 1px solid var(--border);
  }
}
@media (max-width: 700px) {
  .status-actions {
    width: 100%;
  }
}
@media (max-width: 700px) {
  .public-ip-result {
    grid-template-columns: 1fr;
    align-items: flex-start;
  }
  .public-ip-result > .pill {
    justify-self: start;
  }
  .probe-address {
    grid-template-columns: 1fr;
  }
  .probe-address .location-value {
    align-self: start;
  }
}
@media (max-width: 700px) {
  .public-ip-result:has(.probe-address + .probe-address) {
    grid-template-columns: 1fr;
  }
  .font-scale-row {
    align-items: flex-start;
    flex-direction: column;
    gap: 10px;
  }
}
@media (max-width: 700px) {
  .update-banner {
    align-items: flex-start;
    flex-wrap: wrap;
  }
  .update-banner span {
    flex-basis: calc(100% - 30px);
  }
}
@media (max-width: 700px) {
  .host-info {
    max-width: 90px;
  }
  .header-separator {
    margin: 0 1px;
  }
}
.cloud-intro {
  display: flex;
  align-items: center;
  gap: 13px;
  padding: 18px 20px;
  margin-bottom: 18px;
  border: 1px solid var(--accent-border);
  border-radius: 10px;
  background: var(--accent-soft);
}
.cloud-nav-parent {
  color: var(--fg) !important;
  font-weight: 600;
}
.cloud-nav-parent.active {
  background: transparent !important;
  color: var(--fg) !important;
}
.cloud-nav-parent.active > svg:last-child {
  transform: rotate(90deg);
}
.cloud-nav-children {
  margin: -3px 0 5px 20px;
  padding: 1px 0 2px 8px;
  border-left: 1px solid var(--accent-border);
}
.cloud-nav-children button {
  min-height: 31px;
  padding: 7px 9px;
  font-size: 11px;
}
.cloud-nav-children button.active {
  color: var(--accent);
  background: var(--accent-soft);
}
.cloud-nav-children small {
  margin-left: auto;
  padding: 1px 5px;
  border-radius: 5px;
  background: var(--raised);
  font-size: 9px;
}
.cloud-intro-icon,
.cloud-provider {
  display: grid;
  place-items: center;
  flex: 0 0 auto;
  border-radius: 9px;
}
.cloud-intro-icon {
  width: 40px;
  height: 40px;
  color: var(--accent);
  background: var(--surface);
}
.cloud-intro strong {
  font-size: 13px;
}
.cloud-intro p {
  margin-top: 5px;
  color: var(--muted);
  font-size: 11px;
}
.cloud-tabs {
  display: flex;
  gap: 6px;
  padding: 0 8px;
  margin-bottom: 18px;
  border-bottom: 1px solid var(--border);
}
.cloud-tabs button {
  border: 0;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: var(--muted);
  padding: 10px 7px;
}
.cloud-tabs button.active {
  color: var(--accent);
  border-color: var(--accent);
  font-weight: 600;
}
.cloud-tabs span {
  display: inline-block;
  margin-left: 4px;
  padding: 1px 5px;
  border-radius: 5px;
  background: var(--raised);
  font-size: 10px;
}
.cloud-account-list {
  border-top: 1px solid var(--border);
}
.cloud-account-row {
  display: grid;
  grid-template-columns: 40px minmax(180px, 1.5fr) 120px 130px auto;
  align-items: center;
  gap: 16px;
  padding: 15px 0;
  border-bottom: 1px solid var(--border);
}
.cloud-account-row:last-child {
  border-bottom: 0;
}
.cloud-provider {
  width: 36px;
  height: 36px;
  background: #fff0e2;
  color: #dc6f17;
  font-size: 10px;
  font-weight: 700;
}
.cloud-account-row strong {
  display: block;
  font-size: 12px;
}
.cloud-account-row small,
.cloud-field-label {
  display: block;
  margin-top: 5px;
  color: var(--muted);
  font-size: 10px;
}
.cloud-row-actions {
  display: flex;
  gap: 8px;
}
.cloud-row-actions button {
  padding: 7px 9px;
}
.cloud-empty {
  padding: 44px 15px;
}
.cloud-empty svg {
  color: var(--muted);
}
.cloud-security-filters {
  display: grid;
  grid-template-columns: minmax(220px, 1fr) minmax(220px, 1fr);
  gap: 14px;
  margin: 18px 0;
}
.cloud-security-filters label {
  display: flex;
  flex-direction: column;
  gap: 7px;
  color: var(--muted);
  font-size: 11px;
}
.cloud-security-list {
  border-top: 1px solid var(--border);
}
.cloud-security-row {
  display: grid;
  grid-template-columns: 40px minmax(180px, 1.5fr) minmax(
      140px,
      1fr
    ) 100px auto;
  align-items: center;
  gap: 16px;
  padding: 15px 8px;
  border-bottom: 1px solid var(--border);
}
.cloud-security-row strong,
.cloud-security-row small {
  display: block;
}
.cloud-security-row small {
  margin-top: 4px;
  color: var(--muted);
}
.cloud-security-row.selected {
  background: var(--accent-soft);
}
.cloud-security-row:last-child {
  border-bottom: 0;
}
.managed-description {
  color: var(--accent);
  font-weight: 600;
}
.cloud-managed-list article {
  display: grid;
  grid-template-columns: minmax(260px, 2fr) minmax(130px, 1fr) 90px auto;
  align-items: center;
  gap: 16px;
  padding: 14px 0;
  border-top: 1px solid var(--border);
}
.cloud-managed-list small {
  display: block;
  margin-top: 5px;
  color: var(--muted);
}
.cloud-managed-error {
  grid-column: 1 / -1;
  margin: 0;
  color: var(--danger);
  font-size: 10px;
}
.cloud-managed-target {
  padding: 11px;
  border: 1px solid var(--border);
  border-radius: 7px;
  background: var(--raised);
}
.cloud-managed-target span {
  display: block;
  margin-bottom: 5px;
  color: var(--muted);
  font-size: 10px;
}
.cloud-account-modal label {
  display: flex;
  flex-direction: column;
  gap: 8px;
  color: var(--muted);
  font-size: 12px;
  margin: 15px 0;
}
.cloud-account-modal .section-heading > button {
  padding: 5px;
}
.cloud-account-modal .toolbar {
  justify-content: flex-end;
  margin-top: 20px;
}
.cloud-modal-hint {
  display: flex;
  gap: 7px;
  align-items: flex-start;
  padding: 10px;
  border: 1px solid var(--warning-border);
  border-radius: 7px;
  background: var(--warning-bg);
  color: var(--warning);
  font-size: 10px;
  line-height: 1.6;
}
.cloud-source-field {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 8px;
}
.cloud-source-field button {
  white-space: nowrap;
}
.cloud-source-error {
  color: var(--danger) !important;
  line-height: 1.5;
}
@media (max-width: 700px) {
  .cloud-account-row {
    grid-template-columns: 36px 1fr auto;
    gap: 10px;
  }
  .cloud-account-row > div:nth-of-type(2),
  .cloud-account-row > div:nth-of-type(3) {
    display: none;
  }
  .cloud-security-filters {
    grid-template-columns: 1fr;
  }
  .cloud-security-row {
    grid-template-columns: 36px 1fr auto;
    gap: 10px;
  }
  .cloud-security-row > div:nth-of-type(2),
  .cloud-security-row > div:nth-of-type(3) {
    display: none;
  }
  .cloud-managed-list article {
    grid-template-columns: 1fr auto;
  }
  .cloud-managed-list article > div:nth-of-type(2),
  .cloud-managed-list article > div:nth-of-type(3) {
    display: none;
  }
}
</style>
