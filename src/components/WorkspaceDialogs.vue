<script setup lang="ts">
import { useWorkspaceContext } from '../composables/workspaceContext';
const {
  CircleHelp,
  RefreshCw,
  ShieldCheck,
  X,
  desktop,
  busy,
  cloudAccountDialog,
  cloudAccountForm,
  managedRuleDialog,
  managedRuleForm,
  managedRuleSourceCidr,
  managedRuleSourceLoading,
  managedRuleSourceError,
  error,
  confirmAction,
  contextMenu,
  selectedTargets,
  modeName,
  saveCloudAccount,
  refreshManagedRuleSource,
  createManagedRule,
  save,
  addTargetRules,
  closeConfirm,
  confirm,
} = useWorkspaceContext();
</script>
<template>
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

</template>
