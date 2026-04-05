<template>
  <Teleport to="body">
    <Transition name="palette-fade">
      <div v-if="visible" class="fixed inset-0 z-[100] flex justify-center" @mousedown.self="close">
        <div
          class="absolute inset-0 bg-black/30 dark:bg-black/50 backdrop-blur-sm"
          @click="close"
        ></div>
        <div
          class="relative mt-[20vh] w-full max-w-lg h-fit bg-white dark:bg-gray-800 rounded-xl shadow-2xl border border-gray-200 dark:border-gray-700 overflow-hidden"
        >
          <!-- Search input -->
          <div class="flex items-center px-4 border-b border-gray-200 dark:border-gray-700">
            <svg
              class="w-4 h-4 text-gray-400 flex-shrink-0"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
              />
            </svg>
            <input
              ref="inputRef"
              v-model="query"
              type="text"
              :placeholder="$t('commandPalette.placeholder')"
              class="w-full px-3 py-3 text-sm bg-transparent outline-none text-gray-900 dark:text-gray-100 placeholder-gray-400"
              @keydown.down.prevent="moveSelection(1)"
              @keydown.up.prevent="moveSelection(-1)"
              @keydown.enter.prevent="executeSelected"
              @keydown.escape.prevent="close"
            />
            <kbd
              class="hidden sm:inline-flex items-center px-1.5 py-0.5 text-[10px] font-mono text-gray-400 border border-gray-200 dark:border-gray-600 rounded"
            >
              ESC
            </kbd>
          </div>

          <!-- Results -->
          <div class="max-h-[50vh] overflow-y-auto p-2">
            <template v-if="grouped.length > 0">
              <div v-for="group in grouped" :key="group.category">
                <div
                  class="px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-gray-400 dark:text-gray-500"
                >
                  {{ group.category }}
                </div>
                <button
                  v-for="item in group.items"
                  :key="item.id"
                  class="w-full flex items-center justify-between px-3 py-2 rounded-lg text-sm transition-colors"
                  :class="
                    item.id === selectedId
                      ? 'bg-blue-50 dark:bg-blue-500/20 text-blue-700 dark:text-blue-300'
                      : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700/50'
                  "
                  @click="execute(item)"
                  @mouseenter="selectedId = item.id"
                >
                  <span>{{ item.label }}</span>
                  <kbd
                    v-if="item.keys"
                    class="text-[10px] font-mono text-gray-400 dark:text-gray-500 border border-gray-200 dark:border-gray-600 rounded px-1.5 py-0.5"
                  >
                    {{ formatKeys(item.keys) }}
                  </kbd>
                </button>
              </div>
            </template>
            <div v-else class="px-3 py-6 text-center text-sm text-gray-400">
              {{ $t('commandPalette.noResults') }}
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { getAllShortcuts } from '@/composables/useKeyboardShortcuts'

const visible = ref(false)
const query = ref('')
const selectedId = ref('')
const inputRef = ref<HTMLInputElement | null>(null)

function open() {
  visible.value = true
  query.value = ''
  selectedId.value = ''
  nextTick(() => inputRef.value?.focus())
}

function close() {
  visible.value = false
}

function toggle() {
  if (visible.value) close()
  else open()
}

defineExpose({ open, close, toggle, visible })

interface PaletteItem {
  id: string
  keys: string
  label: string
  category: string
  action: () => void
}

const allItems = computed<PaletteItem[]>(() => {
  return getAllShortcuts().filter((s) => s.id !== 'command-palette-open')
})

const filtered = computed<PaletteItem[]>(() => {
  if (!query.value.trim()) return allItems.value
  const q = query.value.toLowerCase()
  return allItems.value.filter(
    (item) => item.label.toLowerCase().includes(q) || item.category.toLowerCase().includes(q),
  )
})

interface GroupedItems {
  category: string
  items: PaletteItem[]
}

const grouped = computed<GroupedItems[]>(() => {
  const map = new Map<string, PaletteItem[]>()
  for (const item of filtered.value) {
    const list = map.get(item.category) || []
    list.push(item)
    map.set(item.category, list)
  }
  return Array.from(map.entries()).map(([category, items]) => ({ category, items }))
})

const flatItems = computed(() => grouped.value.flatMap((g) => g.items))

watch(filtered, () => {
  if (flatItems.value.length > 0 && !flatItems.value.find((i) => i.id === selectedId.value)) {
    selectedId.value = flatItems.value[0].id
  }
})

function moveSelection(delta: number) {
  const items = flatItems.value
  if (items.length === 0) return
  const idx = items.findIndex((i) => i.id === selectedId.value)
  const next = Math.max(0, Math.min(items.length - 1, idx + delta))
  selectedId.value = items[next].id
}

function executeSelected() {
  const item = flatItems.value.find((i) => i.id === selectedId.value)
  if (item) execute(item)
}

function execute(item: PaletteItem) {
  close()
  item.action()
}

function formatKeys(keys: string): string {
  return keys
    .split('+')
    .map((k) => {
      if (k === 'ctrl') return 'Ctrl'
      if (k === 'shift') return 'Shift'
      if (k === 'alt') return 'Alt'
      if (k === 'escape') return 'Esc'
      if (k === ',') return ','
      return k.toUpperCase()
    })
    .join('+')
}
</script>

<style scoped>
.palette-fade-enter-active {
  transition: opacity 0.15s ease;
}
.palette-fade-leave-active {
  transition: opacity 0.1s ease;
}
.palette-fade-enter-from,
.palette-fade-leave-to {
  opacity: 0;
}
</style>
