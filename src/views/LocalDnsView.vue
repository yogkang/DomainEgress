<script setup lang="ts">
import { useWorkspaceContext } from '../composables/workspaceContext';
const {
  ref,
  AlertTriangle,
  Check,
  CircleHelp,
  FileKey2,
  Plus,
  RefreshCw,
  Server,
  Trash2,
  desktop,
  busy,
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
  loadHostGroups,
  loadSystemHosts,
  selectSystemHosts,
  selectDnsGroup,
  saveDnsGroup,
  toggleDnsGroup,
  createDnsGroup,
  deleteDnsGroup,
  selectedDnsEntries,
} = useWorkspaceContext();
</script>
<template>

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
          <button :disabled="busy || !desktop || !selectedDnsGroupDirty" @click="saveDnsGroup">
            <Check :size="15" />应用更改</button
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
