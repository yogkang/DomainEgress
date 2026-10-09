<script setup lang="ts">
import { useWorkspaceContext } from '../composables/workspaceContext';
const {
  Copy,
  Globe2,
  Plus,
  Search,
  ShieldCheck,
  Trash2,
  config,
  draft,
  search,
  ruleSort,
  ruleSearch,
  confirmAction,
  modeName,
  activeRules,
  timestamps,
  orderedRules,
  ruleDate,
  addRules,
  removeRule,
  copyRules,
} = useWorkspaceContext();
</script>
<template>

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
