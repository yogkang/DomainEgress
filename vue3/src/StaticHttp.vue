<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { call, desktop } from './api'
interface Service { port: number; host: string; root: string }
interface Log { timestamp: string; client: string; port: number; method: string; path: string; status: number }
const services = ref<Service[]>([]), running = ref<number[]>([]), logs = ref<Log[]>([])
const draft = ref<Service>({ port: 8080, host: '127.0.0.1', root: '' })
const message = ref(''), busy = ref(false), dragging = ref(false)
let timer: ReturnType<typeof setInterval> | undefined, unlisten: (() => void) | undefined, disposed = false
async function refresh() {
  if (!desktop) return
  const state = await call<{ services: Service[]; running: number[]; logs: Log[] }>('static_http_snapshot')
  services.value = state.services; running.value = state.running; logs.value = state.logs.reverse()
}
async function action(command: string, args?: Record<string, unknown>) {
  busy.value = true
  try { await call(command, args); await refresh(); message.value = '操作成功' }
  catch (error) { message.value = String(error) }
  finally { busy.value = false }
}
async function pick() {
  busy.value = true
  try { const path = await call<string | null>('static_http_pick_directory'); if (path) draft.value.root = path }
  catch (error) { message.value = String(error) }
  finally { busy.value = false }
}
function edit(service: Service) { draft.value = { ...service }; message.value = '编辑后点击保存；运行中的服务会立即使用新根目录。' }
function url(service: Service) { const host = service.host.includes(':') ? `[${service.host}]` : service.host; return `http://${host === '0.0.0.0' ? '127.0.0.1' : host === '[::]' ? '[::1]' : host}:${service.port}/` }
onMounted(async () => {
  try {
    await refresh()
    if (desktop && !disposed) {
      unlisten = await getCurrentWebview().onDragDropEvent(event => {
        dragging.value = event.payload.type === 'over' || event.payload.type === 'enter'
        if (event.payload.type === 'drop') {
          if (event.payload.paths.length !== 1) { message.value = '请一次拖入一个目录'; return }
          draft.value.root = event.payload.paths[0] ?? ''; message.value = '目录已填入，请选择端口并保存配置。'
        }
      })
      if (disposed) unlisten()
      else timer = setInterval(() => void refresh().catch(error => { message.value = String(error) }), 1500)
    }
  } catch (error) { message.value = String(error) }
})
onUnmounted(() => { disposed = true; clearInterval(timer); unlisten?.() })
</script>
<template>
  <section class="static-http">
    <div class="panel" :class="{ dragging }">
      <h2>服务配置</h2>
      <p>拖动本地目录到此页面，或选择目录。每个端口对应一个根目录，保存后立即应用目录修改。</p>
      <div class="static-fields">
        <label>监听地址<input v-model="draft.host" placeholder="127.0.0.1" :disabled="busy" /></label>
        <label>端口<input v-model.number="draft.port" type="number" min="1" max="65535" :disabled="busy" /></label>
        <label class="root">静态资源根目录<input v-model="draft.root" placeholder="输入或拖入目录的绝对路径" :disabled="busy" /></label>
      </div>
      <div class="static-actions"><button :disabled="busy" @click="pick">选择目录</button><button class="primary" :disabled="busy || !draft.root" @click="action('static_http_save', { config: draft })">保存 / 应用目录</button></div>
      <p>默认仅本机访问；设置 0.0.0.0 可供局域网访问。配置会保留，服务需手动启动；目录首页为 index.html。</p>
      <p role="status">{{ message }}</p>
    </div>
    <div class="panel">
      <h2>已配置服务</h2>
      <p v-if="!services.length">暂无服务，请先配置目录和端口。</p>
      <div v-for="service in services" :key="service.port" class="static-service">
        <div><strong>{{ url(service) }}</strong> · {{ running.includes(service.port) ? '运行中' : '已停止' }}<p>{{ service.root }}</p></div>
        <div class="static-actions"><button :disabled="busy" @click="edit(service)">修改目录</button><button :disabled="busy" @click="action(running.includes(service.port) ? 'static_http_stop' : 'static_http_start', { port: service.port })">{{ running.includes(service.port) ? '停止' : '启动' }}</button><button :disabled="busy" @click="action('static_http_remove', { port: service.port })">删除</button></div>
      </div>
    </div>
    <div class="panel"><div class="static-actions"><h2>客户端访问日志</h2><button :disabled="busy" @click="action('static_http_clear_logs')">清空日志</button></div><p>实时显示最近 1000 条请求，同时打印至应用标准输出。</p>
      <div class="table-wrap"><table><thead><tr><th>时间</th><th>客户端</th><th>端口</th><th>方法</th><th>路径</th><th>状态</th></tr></thead><tbody><tr v-for="(log, index) in logs" :key="index"><td>{{ log.timestamp }}</td><td>{{ log.client }}</td><td>{{ log.port }}</td><td>{{ log.method }}</td><td class="request-path">{{ log.path }}</td><td>{{ log.status }}</td></tr></tbody></table></div><p v-if="!logs.length">暂无访问记录。</p>
    </div>
  </section>
</template>
<style scoped>
.static-http { display: grid; gap: 20px; }.panel { padding: 24px; border: 1px solid var(--border, #ddd); border-radius: 14px; }.panel.dragging { outline: 3px solid #4d88ff; }.static-fields { display: flex; flex-wrap: wrap; gap: 16px; margin: 20px 0; }.static-fields label { display: grid; gap: 8px; }.static-fields .root { flex: 1; min-width: 240px; }input { width: 100%; padding: 10px; }.static-actions { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; }.static-actions h2 { margin-right: auto; }button { padding: 8px 14px; cursor: pointer; }.static-service { padding: 16px 0; border-bottom: 1px solid var(--border, #ddd); display: flex; flex-wrap: wrap; justify-content: space-between; gap: 16px; }.static-service p, .request-path { overflow-wrap: anywhere; }table { width: 100%; }td, th { padding: 10px; text-align: left; }.table-wrap { overflow: auto; }
</style>
