import { onMounted, onUnmounted } from 'vue'

export interface ShortcutEntry {
  id: string
  keys: string
  label: string
  category: string
  action: () => void
  when?: () => boolean
}

const shortcuts = new Map<string, ShortcutEntry>()
let listenerAttached = false

function normalizeKeys(e: KeyboardEvent): string {
  const parts: string[] = []
  if (e.ctrlKey || e.metaKey) parts.push('ctrl')
  if (e.shiftKey) parts.push('shift')
  if (e.altKey) parts.push('alt')

  let key = e.key.toLowerCase()
  if (key === ' ') key = 'space'
  if (key === ',') key = ','
  if (key === 'escape') key = 'escape'
  if (!['control', 'shift', 'alt', 'meta'].includes(key)) {
    parts.push(key)
  }
  return parts.join('+')
}

function handleKeydown(e: KeyboardEvent) {
  const combo = normalizeKeys(e)
  const entry = shortcuts.get(combo)
  if (!entry) return
  if (entry.when && !entry.when()) return

  // Don't intercept if user is typing in an input/textarea (unless it's Escape)
  const tag = (e.target as HTMLElement)?.tagName
  if ((tag === 'INPUT' || tag === 'TEXTAREA') && combo !== 'escape') return

  e.preventDefault()
  e.stopPropagation()
  entry.action()
}

function attachListener() {
  if (listenerAttached) return
  document.addEventListener('keydown', handleKeydown, true)
  listenerAttached = true
}

function detachListener() {
  if (!listenerAttached) return
  document.removeEventListener('keydown', handleKeydown, true)
  listenerAttached = false
}

export function registerShortcut(entry: ShortcutEntry) {
  shortcuts.set(entry.keys, entry)
  attachListener()
}

export function unregisterShortcut(keys: string) {
  shortcuts.delete(keys)
  if (shortcuts.size === 0) detachListener()
}

export function getAllShortcuts(): ShortcutEntry[] {
  return Array.from(shortcuts.values())
}

export function useKeyboardShortcuts() {
  onMounted(() => {
    attachListener()
  })

  onUnmounted(() => {
    // Keep listener alive — other components may still have shortcuts
  })

  return { registerShortcut, unregisterShortcut, getAllShortcuts }
}
