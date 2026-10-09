<script setup lang="ts">
import { useWorkspaceContext } from '../composables/workspaceContext';
import SettingsSections from '../components/SettingsSections.vue';
const {
  Check,
  ChevronLeft,
  ChevronRight,
  Plus,
  tab,
  config,
  busy,
  sshForwardPorts,
  showGlobalOptions,
  expandedRuleOptions,
  optionParts,
  addGlobalOption,
  updateGlobalOption,
  removeGlobalOption,
  toggleRuleOptions,
  addRuleOption,
  updateRuleOption,
  removeRuleOption,
  groupedSshForwards,
  isForwardGroupExpanded,
  toggleForwardGroup,
  selectedForwardGroup,
  forwardMenuCollapsed,
  selectedForwardRules,
  selectForwardGroup,
  toggle,
  addSshForward,
  testSshForward,
  testAllSshForwards,
  removeSshForward,
  toggleSshForward,
} = useWorkspaceContext();
</script>
<template>
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
<SettingsSections />

</template>
