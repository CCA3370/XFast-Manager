<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted, watch } from 'vue'
import { useContextMenu } from '@/composables/useContextMenu'

const { visible, x, y, items, hide, handleAction } = useContextMenu()

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
watch(visible, (val) => {
  if (val) {
    nextTick(() => {
      if (!menuRef.value) return
      const rect = menuRef.value.getBoundingClientRect()
      const vw = window.innerWidth
      const vh = window.innerHeight

      if (x.value + rect.width > vw) {
        x.value = vw - rect.width - 4
      }
      if (y.value + rect.height > vh) {
        y.value = vh - rect.height - 4
      }
      if (x.value < 0) x.value = 4
      if (y.value < 0) y.value = 4
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
          class="fixed min-w-[160px] py-1 rounded-lg shadow-xl border
            bg-white dark:bg-gray-800
            border-gray-200 dark:border-gray-600
            text-sm select-none"
          :style="{ left: x + 'px', top: y + 'px' }"
        >
          <template v-for="item in items" :key="item.id">
            <button
              class="w-full flex items-center gap-2 px-3 py-1.5 text-left transition-colors"
              :class="[
                item.disabled
                  ? 'opacity-40 cursor-not-allowed'
                  : item.danger
                    ? 'text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30'
                    : 'text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700'
              ]"
              :disabled="item.disabled"
              @click="!item.disabled && handleAction(item.id)"
            >
              <span
                v-if="item.icon"
                class="w-4 h-4 flex-shrink-0 flex items-center justify-center"
                v-html="item.icon"
              />
              <span class="truncate">{{ item.label }}</span>
            </button>
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
  transition: opacity 0.12s ease, transform 0.12s ease;
}
.ctx-menu-leave-active {
  transition: opacity 0.08s ease, transform 0.08s ease;
}
.ctx-menu-enter-from {
  opacity: 0;
  transform: scale(0.95);
}
.ctx-menu-leave-to {
  opacity: 0;
  transform: scale(0.95);
}
</style>
