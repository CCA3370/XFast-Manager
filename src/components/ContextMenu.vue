<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted, watch } from 'vue'
import { useContextMenu } from '@/composables/useContextMenu'

const { visible, x, y, items, submenuDirection, hide, handleAction } = useContextMenu()

const menuRef = ref<HTMLElement | null>(null)

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && visible.value) {
    hide()
  }
}

function onScrollOrResize() {
  if (visible.value) {
    hide()
  }
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown)
  window.addEventListener('scroll', onScrollOrResize, true)
  window.addEventListener('resize', onScrollOrResize)
})

onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown)
  window.removeEventListener('scroll', onScrollOrResize, true)
  window.removeEventListener('resize', onScrollOrResize)
})

// Viewport boundary clamping
const EDGE_MARGIN = 16

watch(visible, (val) => {
  if (val) {
    nextTick(() => {
      if (!menuRef.value) return
      const rect = menuRef.value.getBoundingClientRect()
      const vw = window.innerWidth
      const vh = window.innerHeight
      const maxH = vh - EDGE_MARGIN * 2

      if (x.value + rect.width > vw) {
        x.value = vw - rect.width - EDGE_MARGIN
      }
      if (x.value < EDGE_MARGIN) x.value = EDGE_MARGIN

      submenuDirection.value =
        x.value + rect.width * 2 > vw - EDGE_MARGIN && x.value - rect.width > EDGE_MARGIN
          ? 'left'
          : 'right'

      // Clamp height and enable scrolling if menu is taller than viewport
      if (rect.height > maxH) {
        menuRef.value.style.maxHeight = `${maxH}px`
        menuRef.value.style.overflowY = 'auto'
      }

      if (y.value + rect.height > vh - EDGE_MARGIN) {
        y.value = vh - rect.height - EDGE_MARGIN
      }
      if (y.value < EDGE_MARGIN) y.value = EDGE_MARGIN
    })
  }
})
</script>

<template>
  <Teleport to="body">
    <Transition name="ctx-menu">
      <div
        v-if="visible"
        class="fixed inset-0 z-[1000]"
        @mousedown.self="hide"
        @contextmenu.prevent="hide"
      >
        <div
          ref="menuRef"
          class="fixed min-w-[160px] py-1 rounded-lg shadow-xl border bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-600 text-sm select-none"
          :style="{ left: x + 'px', top: y + 'px' }"
        >
          <template v-for="item in items" :key="item.id">
            <div class="context-menu-wrapper relative">
              <button
                type="button"
                class="w-full flex items-center gap-2 px-3 py-1.5 text-left transition-colors"
                :class="[
                  item.disabled
                    ? 'opacity-40 cursor-not-allowed'
                    : item.danger
                      ? 'text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30'
                      : 'text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700',
                ]"
                :disabled="item.disabled"
                @click="!item.disabled && !item.children?.length && handleAction(item.id)"
              >
                <span
                  v-if="item.icon"
                  class="w-4 h-4 flex-shrink-0 flex items-center justify-center"
                  v-html="item.icon"
                />
                <span class="min-w-0 flex-1 truncate">{{ item.label }}</span>
                <svg
                  v-if="item.children?.length"
                  class="w-3.5 h-3.5 flex-shrink-0 text-gray-400"
                  :class="{ 'rotate-180': submenuDirection === 'left' }"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 5l7 7-7 7"
                  />
                </svg>
              </button>
              <div
                v-if="item.children?.length && !item.disabled"
                class="context-submenu absolute top-0 min-w-[180px] py-1 rounded-lg shadow-xl border bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-600 text-sm"
                :class="
                  submenuDirection === 'left'
                    ? 'right-full origin-top-right'
                    : 'left-full origin-top-left'
                "
              >
                <button
                  v-for="child in item.children"
                  :key="child.id"
                  type="button"
                  class="w-full flex items-center gap-2 px-3 py-1.5 text-left transition-colors"
                  :class="[
                    child.disabled
                      ? 'opacity-40 cursor-not-allowed'
                      : child.danger
                        ? 'text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30'
                        : 'text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700',
                  ]"
                  :disabled="child.disabled"
                  @click.stop="!child.disabled && handleAction(child.id)"
                >
                  <span
                    v-if="child.icon"
                    class="w-4 h-4 flex-shrink-0 flex items-center justify-center"
                    v-html="child.icon"
                  />
                  <span class="min-w-0 flex-1 truncate">{{ child.label }}</span>
                </button>
              </div>
            </div>
            <div
              v-if="item.dividerAfter"
              class="my-1 border-t border-gray-200 dark:border-gray-600"
            />
          </template>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.ctx-menu-enter-active {
  transition:
    opacity 0.12s ease,
    transform 0.12s ease;
}
.ctx-menu-leave-active {
  transition:
    opacity 0.08s ease,
    transform 0.08s ease;
}
.ctx-menu-enter-from {
  opacity: 0;
  transform: scale(0.95);
}
.ctx-menu-leave-to {
  opacity: 0;
  transform: scale(0.95);
}

.context-submenu {
  display: none;
  z-index: 1;
}

.context-menu-wrapper:hover > .context-submenu,
.context-menu-wrapper:focus-within > .context-submenu {
  display: block;
}
</style>
