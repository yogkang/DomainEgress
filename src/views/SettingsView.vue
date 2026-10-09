<script setup lang="ts">
import { useWorkspaceContext } from '../composables/workspaceContext';
import SettingsSections from '../components/SettingsSections.vue';
const {
  Check,
  appearance,
  theme,
  themes,
  appearanceError,
  setAppearance,
  setTheme,
  desktop,
  tab,
  config,
  error,
  fontScaleChoices,
  autostartEnabled,
  autostartBusy,
  adjustFontScale,
  toggleAutostart,
} = useWorkspaceContext();
</script>
<template>
<section v-if="tab === 'settings'" class="panel font-scale-panel">
            <div class="section-heading">
              <div>
                <h3>文字大小</h3>
                <p>全局应用显示大小；保存配置后会同步到 iCloud。</p>
              </div>
            </div>
            <div class="font-scale-row">
              <div class="font-scale-buttons">
                <button
                  aria-label="缩小文字"
                  :disabled="config.font_scale === fontScaleChoices[0]"
                  @click="adjustFontScale(-1)"
                >
                  −</button
                ><output aria-live="polite">{{ config.font_scale }}%</output
                ><button
                  aria-label="放大文字"
                  :disabled="
                    config.font_scale ===
                    fontScaleChoices[fontScaleChoices.length - 1]
                  "
                  @click="adjustFontScale(1)"
                >
                  +
                </button>
              </div>
              <p class="footnote">
                快捷键：Control + 放大，Control - 缩小，Control 0 恢复 100%。
              </p>
            </div>
          </section>
<section v-if="tab === 'settings'" class="panel">
            <div class="section-heading">
              <div>
                <h3>启动行为</h3>
                <p>
                  登录电脑后自动启动 DomainEgress；关闭后可随时从这里重新开启。
                </p>
              </div>
              <button
                class="primary"
                :disabled="autostartBusy || !desktop"
                @click="toggleAutostart"
              >
                {{
                  autostartBusy
                    ? "处理中…"
                    : autostartEnabled
                      ? "关闭开机启动"
                      : "开启开机启动"
                }}
              </button>
            </div>
            <p class="footnote">
              当前状态：{{
                autostartEnabled ? "已开启" : "未开启"
              }}。此设置写入当前用户的系统登录启动项。
            </p>
          </section>
<section v-if="tab === 'settings'" class="panel appearance-panel">
            <div class="section-heading">
              <div>
                <h3>外观与主题</h3>
                <p>即时生效并自动保存到本机，无需点击“保存配置”。</p>
              </div>
            </div>
            <div class="appearance-modes" role="group" aria-label="外观模式">
              <button
                v-for="mode in ['system', 'light', 'dark'] as const"
                :key="mode"
                :aria-pressed="appearance === mode"
                :class="{ selected: appearance === mode }"
                @click="setAppearance(mode)"
              >
                {{
                  { system: "跟随系统", light: "浅色模式", dark: "暗黑模式" }[
                    mode
                  ]
                }}
              </button>
            </div>
            <div class="theme-grid" role="group" aria-label="程序员颜色主题">
              <button
                v-for="item in themes"
                :key="item.id"
                class="theme-card"
                :class="{ selected: theme === item.id }"
                :aria-pressed="theme === item.id"
                :aria-label="`使用 ${item.name} 主题`"
                @click="setTheme(item.id)"
              >
                <span
                  class="theme-preview"
                  :style="{ background: item.bg, color: item.dark }"
                  ><span>&lt;/&gt;</span
                  ><i :style="{ background: item.dark }"></i
                  ><i :style="{ background: item.light }"></i
                ></span>
                <span class="theme-name"
                  >{{ item.name
                  }}<Check v-if="theme === item.id" :size="15" /></span
                ><small>{{ item.description }}</small>
              </button>
            </div>
            <p v-if="appearanceError" class="appearance-error" role="status">
              {{ appearanceError }}
            </p>
          </section>
<SettingsSections />

</template>
