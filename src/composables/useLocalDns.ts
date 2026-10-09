import { computed, nextTick, ref, watch } from 'vue';
import { call, desktop, type HostMapping, type HostGroup } from '../api';
export function useLocalDns({ notify, action }: {
    notify: (message: string, failed?: boolean) => void;
    action: (fn: () => Promise<void>) => Promise<void>;
}) {
    const hostMappings = ref<HostMapping[]>([]), hostsLoading = ref(false);
    const hostForm = ref({ ip: "", domains: "", group: "" });
    const hostBatchText = ref("");
    const hostBatchGroup = ref("");
    const hostGroups = ref<HostGroup[]>([]);
    const selectedDnsGroup = ref("");
    const hostGroupContent = ref("");
    const selectedDnsGroupDirty = computed(() => {
        const group = hostGroups.value.find((item) => item.name === selectedDnsGroup.value);
        return !!group && hostGroupContent.value !== group.content;
    });
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
    function syncHostsEditorScroll(event: Event, gutter: HTMLElement | null, code: HTMLElement | null) {
        const editor = event.currentTarget as HTMLTextAreaElement;
        if (gutter)
            gutter.scrollTop = editor.scrollTop;
        if (code) {
            code.scrollTop = editor.scrollTop;
            code.scrollLeft = editor.scrollLeft;
        }
    }
    async function loadHostMappings() {
        if (!desktop)
            return;
        hostsLoading.value = true;
        try {
            hostMappings.value = await call<HostMapping[]>("list_host_mappings");
        }
        catch (e) {
            notify(String(e), true);
        }
        finally {
            hostsLoading.value = false;
        }
    }
    async function loadHostGroups() {
        if (!desktop)
            return;
        hostsLoading.value = true;
        try {
            hostGroups.value = await call<HostGroup[]>("list_host_groups");
            const group = hostGroups.value.find((item) => item.name === selectedDnsGroup.value) ||
                hostGroups.value[0];
            if (group)
                selectDnsGroup(group.name);
        }
        catch (e) {
            notify(String(e), true);
        }
        finally {
            hostsLoading.value = false;
        }
    }
    async function loadSystemHosts() {
        if (!desktop)
            return;
        try {
            systemHostsContent.value = await call<string>("read_system_hosts");
        }
        catch (e) {
            notify(String(e), true);
        }
    }
    function selectSystemHosts() {
        selectedDnsView.value = "system";
    }
    function selectDnsGroup(name: string) {
        const group = hostGroups.value.find((item) => item.name === name);
        if (!group)
            return;
        selectedDnsView.value = "group";
        selectedDnsGroup.value = name;
        hostGroupContent.value = group.content;
    }
    async function saveDnsGroup() {
        const group = hostGroups.value.find((item) => item.name === selectedDnsGroup.value);
        if (!group || !selectedDnsGroupDirty.value)
            return;
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
                content: group.name === selectedDnsGroup.value
                    ? hostGroupContent.value
                    : group.content,
                enabled: !group.enabled,
            });
            if (group.name === selectedDnsGroup.value)
                selectDnsGroup(group.name);
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
        if (!window.confirm(`删除分组“${group.name}”及其中的全部映射？`))
            return;
        await action(async () => {
            hostGroups.value = await call<HostGroup[]>("delete_host_group", {
                name: group.name,
            });
            selectedDnsGroup.value = "";
            hostGroupContent.value = "";
            const next = hostGroups.value[0];
            if (next)
                selectDnsGroup(next.name);
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
            if (!value)
                return;
            const tokens = value.split(/\s+/).filter(Boolean);
            let ip = "";
            let domains: string[] = [];
            const looksLikeIp = (token: string) => /^[0-9a-fA-F:.]+$/.test(token) &&
                (token.includes(".") || token.includes(":"));
            if (tokens.length >= 2 && looksLikeIp(tokens[0])) {
                ip = tokens[0];
                domains = tokens.slice(1);
            }
            else {
                const match = value.match(/^([^\s=,:]+)\s*(?:=|:)\s*(\S+)$/);
                if (match) {
                    ip = match[2];
                    domains = [match[1]];
                }
                else if (tokens.length >= 2 && looksLikeIp(tokens.at(-1) || "")) {
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
        if (!(event.ctrlKey || event.metaKey) || event.key !== "/")
            return;
        event.preventDefault();
        const textarea = event.currentTarget as HTMLTextAreaElement;
        const value = textarea.value;
        const start = value.lastIndexOf("\n", Math.max(0, textarea.selectionStart - 1)) + 1;
        const nextBreak = value.indexOf("\n", textarea.selectionEnd);
        const end = nextBreak === -1 ? value.length : nextBreak;
        const selected = value.slice(start, end);
        const lines = selected.split("\n");
        const contentLines = lines.filter((line) => line.trim());
        const uncomment = contentLines.length > 0 && contentLines.every((line) => /^\s*#/.test(line));
        const updated = lines
            .map((line) => {
            if (!line.trim())
                return line;
            if (uncomment)
                return line.replace(/^(\s*)#\s?/, "$1");
            return line.replace(/^(\s*)/, "$1# ");
        })
            .join("\n");
        hostBatchText.value = value.slice(0, start) + updated + value.slice(end);
        void nextTick(() => textarea.setSelectionRange(start, start + updated.length));
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
            notify(`已添加 ${mappings.length} 个地址、${mappings.reduce((count, mapping) => count + mapping.domains.length, 0)} 个域名，DNS 缓存已刷新`);
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
        if (!window.confirm(`删除 ${mapping.ip} 的 ${mapping.domains.join(", ")} 映射？`))
            return;
        await action(async () => {
            hostMappings.value = await call<HostMapping[]>("remove_host_mapping", {
                ip: mapping.ip,
                domains: mapping.domains,
            });
            notify("本地 hosts 映射已删除，DNS 缓存已刷新");
        });
    }
    const hostMappingGroups = computed(() => {
        const groups = new Map<string, HostMapping[]>();
        for (const mapping of hostMappings.value) {
            const name = mapping.group.trim() || "未分组";
            groups.set(name, [...(groups.get(name) || []), mapping]);
        }
        return [...groups].map(([name, mappings]) => ({ name, mappings }));
    });
    const selectedHostMappings = computed(() => hostMappingGroups.value.find((group) => group.name === selectedHostGroup.value)?.mappings || []);
    const selectedDnsEntries = computed(() => {
        if (selectedDnsView.value !== "group")
            return [];
        const entries = hostGroupContent.value.split(/\r?\n/).flatMap((line, index) => {
            const value = line.replace(/^\s*#\s?/, "").trim();
            if (!value || value.startsWith("#"))
                return [];
            const [ip, ...domains] = value.split(/\s+/);
            return ip && domains.length ? [{ line: index + 1, ip, domains }] : [];
        });
        const domainCounts = new Map<string, number>();
        entries.forEach((entry) => entry.domains.forEach((domain) => domainCounts.set(domain.toLowerCase(), (domainCounts.get(domain.toLowerCase()) || 0) + 1)));
        return entries.map((entry) => ({ ...entry, duplicate: entry.domains.some((domain) => (domainCounts.get(domain.toLowerCase()) || 0) > 1) }));
    });
    watch(hostMappingGroups, (groups) => {
        if (!groups.some((group) => group.name === selectedHostGroup.value))
            selectedHostGroup.value = groups[0]?.name || "";
    }, { immediate: true });
    return { hostMappings, hostsLoading, hostForm, hostBatchText, hostBatchGroup, hostGroups, selectedDnsGroup, hostGroupContent, selectedDnsGroupDirty, systemHostsContent, selectedDnsView, hostGroupNameDraft, selectedHostGroup, editingHostKey, editingHostOriginal, editingHostForm, systemEditorGutter, systemEditorCode, groupEditorGutter, groupEditorCode, systemEditorLines, groupEditorLines, isHostsComment, syncHostsEditorScroll, loadHostMappings, loadHostGroups, loadSystemHosts, selectSystemHosts, selectDnsGroup, saveDnsGroup, toggleDnsGroup, createDnsGroup, deleteDnsGroup, addHostMapping, parseHostBatch, toggleHostBatchComments, addHostBatch, hostMappingKey, beginEditHostMapping, cancelEditHostMapping, saveHostMappingEdit, removeHostMapping, hostMappingGroups, selectedHostMappings, selectedDnsEntries };
}
