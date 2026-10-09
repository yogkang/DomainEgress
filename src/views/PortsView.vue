<script setup lang="ts">
import { useWorkspaceContext } from '../composables/workspaceContext';
const {
  RefreshCw,
  Search,
  desktop,
  saved,
  search,
  portBusy,
  portSearch,
  confirmAction,
  filteredPorts,
  loadPorts,
} = useWorkspaceContext();
</script>
<template>

<section class="panel">
  <div class="section-heading">
    <div class="search">
      <Search :size="17" /><input
        v-model="portSearch"
        aria-label="搜索端口"
        placeholder="搜索端口、进程或 PID"
      />
    </div>
    <button :disabled="portBusy || !desktop" @click="loadPorts">
      <RefreshCw :size="16" :class="{ spin: portBusy }" />{{
        portBusy ? "查询中…" : "刷新"
      }}
    </button>
  </div>
  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th>监听地址</th>
          <th>进程 / PID</th>
          <th>启动时间 / 已运行</th>
          <th class="right">操作</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="p in filteredPorts" :key="`${p.pid}-${p.port}`">
          <td class="mono">{{ p.port }}</td>
          <td>
            {{ p.name
            }}<small class="cell-sub mono">{{ p.pid }}</small>
          </td>
          <td>
            {{ p.started
            }}<small class="cell-sub">{{ p.elapsed }}</small>
          </td>
          <td class="right">
            <button class="danger-text" @click="confirmAction = p">
              结束进程
            </button>
          </td>
        </tr>
      </tbody>
    </table>
    <div v-if="!filteredPorts.length" class="empty">
      {{
        portBusy ? "正在读取系统监听端口…" : "没有可显示的监听端口"
      }}
    </div>
  </div>
  <p class="footnote">
    数据来自 macOS lsof，只显示当前用户可见的 TCP 监听进程；当前每
    {{ saved.port_refresh_interval_seconds }} 秒自动刷新。
  </p>
</section>


</template>
