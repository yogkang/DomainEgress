<script setup lang="ts">
import { useWorkspaceContext } from '../composables/workspaceContext';
const {
  CircleHelp,
  Cloud,
  Plus,
  RefreshCw,
  ShieldCheck,
  desktop,
  tabs,
  busy,
  cloudAccounts,
  cloudLoading,
  cloudSubtab,
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
  error,
  loadCloudAccounts,
  openCloudAccountDialog,
  verifyCloudAccount,
  loadCloudRegions,
  loadCloudSecurityGroups,
  selectCloudSecurityGroup,
  loadCloudSecurityRules,
  openManagedRuleDialog,
  syncManagedRule,
  syncAllManagedRules,
  deleteManagedRule,
  deleteCloudAccount,
} = useWorkspaceContext();
</script>
<template>

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
