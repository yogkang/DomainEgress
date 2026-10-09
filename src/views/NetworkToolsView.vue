<script setup lang="ts">
import { useWorkspaceContext } from '../composables/workspaceContext';
const {
  ChevronRight,
  Play,
  Radio,
  Square,
  desktop,
  error,
  networkTool,
  networkProbe,
  networkResult,
  networkBusy,
  networkTools,
  selectNetworkTool,
  detailText,
  runNetworkProbe,
  cancelNetworkProbe,
} = useWorkspaceContext();
</script>
<template>

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
