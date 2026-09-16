import { computed, ref } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'

export type Appearance = 'system' | 'light' | 'dark'
export const themes = [
  { id: 'forest', name: 'Forest', description: '默认森林绿', light: '#238360', dark: '#73c9a5', bg: '#15221d' },
  { id: 'github', name: 'GitHub', description: '简洁蓝色', light: '#0969da', dark: '#58a6ff', bg: '#0d1117' },
  { id: 'dracula', name: 'Dracula', description: '经典紫色', light: '#7545ba', dark: '#bd93f9', bg: '#282a36' },
  { id: 'nord', name: 'Nord', description: '北欧冰蓝', light: '#416985', dark: '#88c0d0', bg: '#2e3440' },
  { id: 'monokai', name: 'Monokai', description: '暖灰荧光绿', light: '#56731a', dark: '#a6e22e', bg: '#272822' },
  { id: 'tokyo', name: 'Tokyo Night', description: '静夜蓝紫', light: '#435bb3', dark: '#7aa2f7', bg: '#1a1b26' },
] as const
export type ThemeId = typeof themes[number]['id']
const storageKey = 'domain-egress.appearance.v1'
export const appearance = ref<Appearance>('system')
export const theme = ref<ThemeId>('github')
export const appearanceError = ref('')
const media = window.matchMedia('(prefers-color-scheme: dark)')
const systemDark = ref(media.matches)
export const resolvedAppearance = computed(() => appearance.value === 'system' ? (systemDark.value ? 'dark' : 'light') : appearance.value)

function apply() {
  const root = document.documentElement
  root.dataset.theme = theme.value
  root.dataset.mode = resolvedAppearance.value
  root.style.colorScheme = resolvedAppearance.value
  if (isTauri()) void getCurrentWindow().setTheme(resolvedAppearance.value).catch(() => {
    appearanceError.value = '界面配色已切换，窗口标题栏配色同步失败。'
  })
}
export function setAppearance(mode: Appearance) { appearance.value = mode; persist() }
export function setTheme(id: ThemeId) { theme.value = id; persist() }
function persist() {
  appearanceError.value = ''
  apply()
  try { localStorage.setItem(storageKey, JSON.stringify({ mode: appearance.value, theme: theme.value })) }
  catch { appearanceError.value = '外观已切换，但无法保存到本机；重启后可能恢复默认。' }
}
export function initAppearance() {
  try {
    const stored = JSON.parse(localStorage.getItem(storageKey) || 'null')
    if (stored && ['light', 'dark', 'system'].includes(stored.mode)) appearance.value = stored.mode
    if (stored && themes.some(t => t.id === stored.theme)) theme.value = stored.theme
  } catch { /* Invalid or unavailable storage falls back to system/GitHub. */ }
  apply()
  media.addEventListener('change', event => { systemDark.value = event.matches; if (appearance.value === 'system') apply() })
}
