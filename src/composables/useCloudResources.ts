import { ref, type Ref } from 'vue';
import { call, desktop, type CloudAccount, type CloudRegion, type CloudSecurityGroup, type CloudSecurityRule, type SaveCloudAccountInput, type CreateManagedRuleInput, type ManagedRuleConfig, type ManagedRuleSyncResult } from '../api';
export function useCloudResources({ notify, action, busy }: {
    busy: Ref<boolean>;
    notify: (message: string, failed?: boolean) => void;
    action: (fn: () => Promise<void>) => Promise<void>;
}) {
    const cloudAccounts = ref<CloudAccount[]>([]), cloudLoading = ref(false), cloudAccountDialog = ref(false), cloudSubtab = ref<"accounts" | "security-groups">("accounts");
    const cloudAccountForm = ref<SaveCloudAccountInput>({
        display_name: "",
        auth_method: "ram_access_key",
        access_key_id: "",
        access_key_secret: "",
    });
    const cloudRegions = ref<CloudRegion[]>([]), cloudSecurityGroups = ref<CloudSecurityGroup[]>([]), cloudRegionLoading = ref(false), cloudGroupsLoading = ref(false), selectedCloudAccountId = ref(""), selectedCloudRegion = ref("");
    const selectedSecurityGroupId = ref(""), cloudSecurityRules = ref<CloudSecurityRule[]>([]), cloudRulesLoading = ref(false), managedRules = ref<ManagedRuleConfig[]>([]), managedRuleDialog = ref(false);
    const managedRuleForm = ref<CreateManagedRuleInput>({
        account_id: "",
        region: "",
        security_group_id: "",
        protocol: "TCP",
        port_range: "22/22",
        priority: 1,
    });
    const managedRuleSourceCidr = ref(""), managedRuleSourceLoading = ref(false), managedRuleSourceError = ref("");
    let cloudRegionRequest = 0, cloudGroupRequest = 0, cloudRuleRequest = 0, managedSourceRequest = 0;
    let cloudSyncRunning = false;
    async function loadCloudAccounts() {
        if (!desktop)
            return;
        cloudLoading.value = true;
        try {
            cloudAccounts.value = await call<CloudAccount[]>("list_cloud_accounts");
        }
        catch (e) {
            notify(String(e), true);
        }
        finally {
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
            notify(`已验证并保存阿里云账号：${account.verified_account_id || account.display_name}`);
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
        const request = ++cloudRegionRequest, accountId = selectedCloudAccountId.value;
        selectedCloudRegion.value = "";
        selectedSecurityGroupId.value = "";
        cloudRegions.value = [];
        cloudSecurityGroups.value = [];
        cloudSecurityRules.value = [];
        if (!accountId)
            return;
        cloudRegionLoading.value = true;
        try {
            const regions = await call<CloudRegion[]>("list_cloud_regions", {
                accountId,
            });
            if (request !== cloudRegionRequest ||
                accountId !== selectedCloudAccountId.value)
                return;
            cloudRegions.value = regions;
            selectedCloudRegion.value = regions[0]?.id || "";
            if (selectedCloudRegion.value)
                await loadCloudSecurityGroups();
        }
        catch (e) {
            if (request === cloudRegionRequest)
                notify(String(e), true);
        }
        finally {
            if (request === cloudRegionRequest)
                cloudRegionLoading.value = false;
        }
    }
    async function loadCloudSecurityGroups() {
        const request = ++cloudGroupRequest, accountId = selectedCloudAccountId.value, region = selectedCloudRegion.value;
        selectedSecurityGroupId.value = "";
        cloudSecurityGroups.value = [];
        cloudSecurityRules.value = [];
        if (!accountId || !region)
            return;
        cloudGroupsLoading.value = true;
        try {
            const groups = await call<CloudSecurityGroup[]>("list_cloud_security_groups", { accountId, region });
            if (request !== cloudGroupRequest || region !== selectedCloudRegion.value)
                return;
            cloudSecurityGroups.value = groups;
        }
        catch (e) {
            if (request === cloudGroupRequest)
                notify(String(e), true);
        }
        finally {
            if (request === cloudGroupRequest)
                cloudGroupsLoading.value = false;
        }
    }
    async function selectCloudSecurityGroup(group: CloudSecurityGroup) {
        selectedSecurityGroupId.value = group.id;
        await loadCloudSecurityRules();
    }
    async function loadCloudSecurityRules() {
        const request = ++cloudRuleRequest, accountId = selectedCloudAccountId.value, region = selectedCloudRegion.value, securityGroupId = selectedSecurityGroupId.value;
        cloudSecurityRules.value = [];
        if (!accountId || !region || !securityGroupId)
            return;
        cloudRulesLoading.value = true;
        try {
            const rules = await call<CloudSecurityRule[]>("list_cloud_security_group_rules", { accountId, region, securityGroupId });
            if (request !== cloudRuleRequest ||
                securityGroupId !== selectedSecurityGroupId.value)
                return;
            cloudSecurityRules.value = rules;
        }
        catch (e) {
            if (request === cloudRuleRequest)
                notify(String(e), true);
        }
        finally {
            if (request === cloudRuleRequest)
                cloudRulesLoading.value = false;
        }
    }
    async function loadManagedRules() {
        if (!desktop)
            return;
        try {
            managedRules.value = await call<ManagedRuleConfig[]>("list_cloud_managed_rules");
        }
        catch (e) {
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
            if (request === managedSourceRequest)
                managedRuleSourceCidr.value = cidr;
        }
        catch (e) {
            if (request === managedSourceRequest)
                managedRuleSourceError.value = String(e);
        }
        finally {
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
            const result = await call<ManagedRuleSyncResult>("create_cloud_managed_rule", { input: managedRuleForm.value });
            managedRuleDialog.value = false;
            await Promise.all([loadManagedRules(), loadCloudSecurityRules()]);
            notify(`受管规则已${result.action === "created" ? "创建" : "更新"}：${result.config.last_source_cidr}`);
        });
        await loadManagedRules();
    }
    async function syncManagedRule(id: string) {
        await action(async () => {
            const result = await call<ManagedRuleSyncResult>("sync_cloud_managed_rule", { id });
            await Promise.all([loadManagedRules(), loadCloudSecurityRules()]);
            notify(`受管规则已同步到 ${result.config.last_source_cidr}`);
        });
    }
    async function syncAllManagedRules(silent = false) {
        if (!desktop ||
            cloudSyncRunning ||
            !managedRules.value.some((rule) => rule.enabled))
            return;
        cloudSyncRunning = true;
        try {
            const results = await call<ManagedRuleSyncResult[]>("sync_all_cloud_managed_rules");
            await loadManagedRules();
            if (selectedSecurityGroupId.value)
                await loadCloudSecurityRules();
            if (!silent && results.length)
                notify(`已同步 ${results.length} 条受管规则`);
        }
        catch (e) {
            await loadManagedRules();
            if (!silent)
                notify(String(e), true);
        }
        finally {
            cloudSyncRunning = false;
        }
    }
    async function deleteManagedRule(rule: ManagedRuleConfig) {
        if (!window.confirm(`删除受管规则“${rule.description}”？对应的阿里云规则也会被撤销。`))
            return;
        busy.value = true;
        try {
            await call("delete_cloud_managed_rule", {
                id: rule.id,
                revokeRemote: true,
            });
            await Promise.all([loadManagedRules(), loadCloudSecurityRules()]);
            notify("受管规则及对应云端规则已删除");
        }
        catch (e) {
            const force = window.confirm(`${String(e)}\n\n是否仅移除本机管理记录？这可能在阿里云中留下仍开放的规则，请稍后到云控制台核对。`);
            if (force) {
                try {
                    await call("delete_cloud_managed_rule", {
                        id: rule.id,
                        revokeRemote: false,
                    });
                    await loadManagedRules();
                    notify("已仅移除本机管理记录；请到阿里云控制台核对并清理可能残留的规则", true);
                }
                catch (localError) {
                    notify(String(localError), true);
                }
            }
            else {
                notify(String(e), true);
            }
        }
        finally {
            busy.value = false;
        }
    }
    function seedCloudPreview() {
        if (desktop)
            return;
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
        if (!window.confirm(`删除云账号“${account.display_name}”？将移除本机保存的凭据。`))
            return;
        await action(async () => {
            await call("delete_cloud_account", { id: account.id });
            await loadCloudAccounts();
            notify(`已删除云账号：${account.display_name}`);
        });
    }
    return { cloudAccounts, cloudLoading, cloudAccountDialog, cloudSubtab, cloudAccountForm, cloudRegions, cloudSecurityGroups, cloudRegionLoading, cloudGroupsLoading, selectedCloudAccountId, selectedCloudRegion, selectedSecurityGroupId, cloudSecurityRules, cloudRulesLoading, managedRules, managedRuleDialog, managedRuleForm, managedRuleSourceCidr, managedRuleSourceLoading, managedRuleSourceError, cloudRegionRequest, cloudGroupRequest, cloudRuleRequest, managedSourceRequest, cloudSyncRunning, loadCloudAccounts, openCloudAccountDialog, saveCloudAccount, verifyCloudAccount, loadCloudRegions, loadCloudSecurityGroups, selectCloudSecurityGroup, loadCloudSecurityRules, loadManagedRules, refreshManagedRuleSource, openManagedRuleDialog, createManagedRule, syncManagedRule, syncAllManagedRules, deleteManagedRule, seedCloudPreview, deleteCloudAccount };
}
