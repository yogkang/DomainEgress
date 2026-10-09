<script setup lang="ts">
import { useWorkspaceContext } from '../composables/workspaceContext';
const {
  ref,
  ArrowUpRight,
  ListFilter,
  Search,
  Trash2,
  desktop,
  logs,
  error,
  search,
  logLevel,
  logOutcome,
  trendRange,
  autoScrollLogs,
  logPanel,
  filteredLogs,
  blockedDomains,
  trendRangeLabel,
  date,
  showTargetMenu,
  clearLogs,
} = useWorkspaceContext();
</script>
<template>

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
