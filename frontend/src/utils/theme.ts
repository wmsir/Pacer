/**
 * 主题：强调色与深浅色模式
 *
 * - 强调色保存在 localStorage `primaryColor`（「自定义主题与水印」页设置）
 * - 深浅色由 @vueuse/core 的 useDark 保存在 `vueuse-color-scheme`
 * 以前只有打开设置页时才会应用这两项，刷新后主题色、深色模式都会丢失；
 * 现在在应用启动时调用 initTheme()，任何页面刷新都保持一致。
 */

export const DEFAULT_PRIMARY = '#2f55d4'
const COLOR_KEY = 'primaryColor'
const SCHEME_KEY = 'vueuse-color-scheme'

type RGB = [number, number, number]

const hexToRgb = (hex: string): RGB | null => {
  const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(hex.trim())
  if (!m) return null
  let h = m[1]
  if (h.length === 3) h = h.split('').map(c => c + c).join('')
  return [0, 2, 4].map(i => parseInt(h.slice(i, i + 2), 16)) as RGB
}

const rgbToHex = (rgb: RGB) => '#' + rgb.map(v => Math.round(Math.max(0, Math.min(255, v))).toString(16).padStart(2, '0')).join('')

/** 把 a 按 weight（0~1）混向 b */
const mix = (a: RGB, b: RGB, weight: number): RGB => [0, 1, 2].map(i => a[i] * (1 - weight) + b[i] * weight) as RGB

export const isDarkMode = () => document.documentElement.classList.contains('dark')

/** 应用强调色：同时生成 Element Plus 需要的明暗梯度和页面使用的 --c-accent 系列变量 */
export function applyPrimaryColor(hex?: string | null) {
  const root = document.documentElement.style
  const base = hexToRgb(hex || '') || hexToRgb(DEFAULT_PRIMARY)!
  const dark = isDarkMode()
  // 深色模式下浅梯度应混向深色底，否则悬停态会变成刺眼的浅色块
  const toward: RGB = dark ? [24, 24, 27] : [255, 255, 255]
  // 深色模式下把强调色本身提亮一些，保证在深底上的对比度
  const main = dark ? mix(base, [255, 255, 255], 0.35) : base

  root.setProperty('--el-color-primary', rgbToHex(main))
  for (const n of [3, 5, 7, 8, 9]) {
    root.setProperty(`--el-color-primary-light-${n}`, rgbToHex(mix(main, toward, n / 10)))
  }
  root.setProperty('--el-color-primary-dark-2', rgbToHex(mix(main, dark ? [255, 255, 255] : [0, 0, 0], 0.2)))

  root.setProperty('--c-accent', rgbToHex(main))
  root.setProperty('--c-accent-hover', rgbToHex(mix(main, dark ? [255, 255, 255] : [0, 0, 0], 0.15)))
  root.setProperty('--c-accent-soft', dark ? `rgba(${main.map(Math.round).join(',')}, 0.14)` : rgbToHex(mix(main, toward, 0.92)))
  root.setProperty('--c-accent-border', dark ? `rgba(${main.map(Math.round).join(',')}, 0.35)` : rgbToHex(mix(main, toward, 0.75)))
}

export function savePrimaryColor(hex: string) {
  localStorage.setItem(COLOR_KEY, hex)
  applyPrimaryColor(hex)
}

export function getSavedPrimaryColor() {
  return localStorage.getItem(COLOR_KEY) || DEFAULT_PRIMARY
}

/** 启动时调用：恢复深浅色与强调色 */
export function initTheme() {
  const scheme = localStorage.getItem(SCHEME_KEY) || 'auto'
  const prefersDark = typeof window.matchMedia === 'function' && window.matchMedia('(prefers-color-scheme: dark)').matches
  const dark = scheme === 'dark' || (scheme === 'auto' && prefersDark)
  document.documentElement.classList.toggle('dark', dark)
  applyPrimaryColor(localStorage.getItem(COLOR_KEY))

  // 深浅色切换后重新计算强调色梯度（useDark 只切换 class）
  const observer = new MutationObserver(() => applyPrimaryColor(localStorage.getItem(COLOR_KEY)))
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })
}
