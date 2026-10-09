<script setup lang="ts">
import { provide } from 'vue';
import { useWorkspace } from './composables/useWorkspace';
import { workspaceKey } from './composables/workspaceContext';
import StaticHttp from './views/StaticHttpView.vue';
import NetworkToolsView from './views/NetworkToolsView.vue';
import LocalDnsView from './views/LocalDnsView.vue';
import CloudView from './views/CloudView.vue';
import OverviewView from './views/OverviewView.vue';
import RulesView from './views/RulesView.vue';
import LogsView from './views/LogsView.vue';
import PortsView from './views/PortsView.vue';
import SettingsView from './views/SettingsView.vue';
import SshForwardView from './views/SshForwardView.vue';
import WorkspaceDialogs from './components/WorkspaceDialogs.vue';
const workspace = useWorkspace();
provide(workspaceKey, workspace);
const {
  Check,
  ChevronRight,
  CircleHelp,
  Cloud,
  Globe2,
  RefreshCw,
  X,
  desktop,
  tabs,
  tab,
  config,
  pageTitle,
  pageEyebrow,
  pageDescription,
  running,
  logs,
  busy,
  connected,
  initialized,
  hostname,
  cloudAccounts,
  cloudSubtab,
  updateInfo,
  updateBusy,
  updateDismissed,
  notice,
  error,
  draft,
  undoRule,
  ports,
  dirty,
  forwardMenuCollapsed,
  localAddressSummary,
  handleCopyClick,
  checkUpdate,
  openUpdate,
  refresh,
  save,
  discard,
  undoLastRule,
  navigate,
} = workspace;
</script>
<template>
<div
  class="app-shell"
  :style="{ '--font-scale': config.font_scale / 100 }"
  @click="handleCopyClick"
>
  <aside class="sidebar">
    <div class="brand">
      <span class="brand-mark"><Globe2 :size="24" /></span>
      <div>DomainEgress<small>本地网络访问控制</small></div>
    </div>
    <div class="nav-label">工作空间</div>
    <nav aria-label="主导航">
      <button
        v-for="item in tabs.filter((item) => item.id !== 'settings')"
        :key="item.id"
        :class="{ active: tab === item.id }"
        @click="navigate(item.id)"
      >
        <component :is="item.icon" :size="18" /><span>{{ item.title }}</span
        ><ChevronRight v-if="tab === item.id" :size="15" />
      </button>
      <button
        class="cloud-nav-parent"
        :class="{ active: tab === 'cloud' }"
        @click="navigate('cloud')"
      >
        <Cloud :size="18" /><span>云资源</span><ChevronRight :size="15" />
      </button>
      <div v-if="tab === 'cloud'" class="cloud-nav-children">
        <button
          :class="{ active: cloudSubtab === 'accounts' }"
          @click="cloudSubtab = 'accounts'"
        >
          云账号
          <small v-if="cloudAccounts.length">{{
            cloudAccounts.length
          }}</small>
        </button>
        <button
          :class="{ active: cloudSubtab === 'security-groups' }"
          @click="cloudSubtab = 'security-groups'"
        >
          安全组
        </button>
      </div>
      <button
        v-for="item in tabs.filter((item) => item.id === 'settings')"
        :key="item.id"
        :class="{ active: tab === item.id }"
        @click="navigate(item.id)"
      >
        <component :is="item.icon" :size="18" /><span>{{ item.title }}</span
        ><ChevronRight v-if="tab === item.id" :size="15" />
      </button>
    </nav>
    <div class="sidebar-bottom">
      <div class="local-badge">
        <span class="dot" :class="{ live: running }"></span
        >{{ running ? "代理正在运行" : "代理已停止" }}
      </div>
      <p>本地网络安全代理</p>
      <span class="version">DESKTOP / 0.5.0</span
      ><button
        class="sidebar-update"
        :disabled="updateBusy || !desktop"
        @click="checkUpdate"
      >
        <RefreshCw
          :size="13"
          :class="['refresh-icon', { 'is-spinning': updateBusy }]"
        />{{ updateBusy ? "检查中…" : "检查更新" }}</button
      ><small v-if="updateInfo?.available" class="sidebar-update-hint"
        >发现新版本 v{{ updateInfo.latest_version }}</small
      ><small
        v-else-if="updateInfo && !updateInfo.error"
        class="sidebar-update-hint"
        >当前已是最新版本</small
      >
    </div>
  </aside>
  <main>
    <header>
      <div class="breadcrumb">
        工作空间 <ChevronRight :size="13" /> <span>{{ pageTitle }}</span>
      </div>
      <div class="header-right">
        <span class="desktop-label">{{
          desktop ? "本机桌面" : "界面预览"
        }}</span
        ><span class="dot" :class="{ live: connected && desktop }"></span
        >{{ desktop ? (connected ? "核心已连接" : "连接中断") : "未连接核心"
        }}<span class="header-separator"></span
        ><span class="host-info" :title="hostname">主机名 {{ hostname }}</span
        ><span class="header-separator"></span
        ><span class="host-info" :title="localAddressSummary"
          >IPv4 {{ localAddressSummary }}</span
        >
      </div>
    </header>
    <div
      class="content"
      :class="{
        'forward-view': tab === 'ssh-forward',
        'forward-menu-collapsed': forwardMenuCollapsed,
      }"
    >
      <div v-if="!desktop" class="preview-banner">
        <CircleHelp :size="17" />
        当前为浏览器界面预览。代理操作、配置保存与端口查询请使用桌面应用。
      </div>
      <div class="page-heading">
        <div>
          <div class="eyebrow">{{ pageEyebrow }}</div>
          <h1>{{ pageTitle }}</h1>
          <p>{{ pageDescription }}</p>
        </div>
        <div class="page-heading-actions">
          <button
            v-if="
              tab === 'rules' ||
              tab === 'settings' ||
              tab === 'ssh-forward' ||
              (tab === 'overview' && dirty)
            "
            class="primary"
            :disabled="
              busy || !desktop || !initialized || !connected || !dirty
            "
            @click="save"
          >
            <Check :size="16" />保存配置<span
              v-if="dirty"
              class="unsaved"
            ></span></button
          ><span v-if="tab === 'overview'" class="pill"
            >HTTP / HTTPS / SOCKS5</span
          >
        </div>
      </div>
      <div v-if="notice" class="notice" :class="{ error }" role="status">
        <span>{{ notice }}</span
        ><button v-if="undoRule" class="undo" @click="undoLastRule">
          撤销</button
        ><button aria-label="关闭提示" @click="notice = ''">
          <X :size="16" />
        </button>
      </div>
      <div v-if="dirty" class="draft-banner">
        有未保存的修改，保存后生效。<button @click="discard">撤销修改</button>
      </div>
      <div
        v-if="updateInfo?.available && !updateDismissed"
        class="update-banner"
      >
        <RefreshCw :size="17" /><span
          >发现新版本 <strong>v{{ updateInfo.latest_version }}</strong
          >，当前版本 v{{ updateInfo.current_version }}。</span
        ><button class="primary" @click="openUpdate">查看更新</button
        ><button
          class="update-dismiss"
          aria-label="稍后提醒"
          @click="updateDismissed = true"
        >
          稍后
        </button>
      </div>

      <StaticHttp v-if="tab === 'static-http'" />
      <NetworkToolsView v-if="tab === 'network-tools'" />
      <LocalDnsView v-if="tab === 'local-dns'" />
      <CloudView v-if="tab === 'cloud'" />


      <OverviewView v-if="tab === 'overview'" />

      <RulesView v-if="tab === 'rules'" />

      <LogsView v-if="tab === 'logs'" />

      <PortsView v-if="tab === 'ports'" />

      <SettingsView v-if="tab === 'settings'" />
      <SshForwardView v-if="tab === 'ssh-forward'" />
    </div>
  </main>


  <WorkspaceDialogs />
</div>

</template>
