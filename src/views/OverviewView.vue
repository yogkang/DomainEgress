<script setup lang="ts">
import { useWorkspaceContext } from '../composables/workspaceContext';
const {
  ArrowUpRight,
  CircleHelp,
  CircleStop,
  Globe2,
  Network,
  Play,
  RefreshCw,
  ShieldCheck,
  desktop,
  config,
  saved,
  running,
  busy,
  connected,
  interfaces,
  publicIpBusy,
  error,
  trendRange,
  trendInterval,
  egressProbes,
  buckets,
  maximum,
  trendRangeLabel,
  admitted,
  blocked,
  locationLabel,
  probePublicIp,
  refresh,
  toggle,
  navigate,
} = useWorkspaceContext();
</script>
<template>

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
