<template>
  <Teleport to="body">
    <div
      :id="id"
      ref="panel"
      class="fixed z-[1001] overflow-y-auto overscroll-contain rounded-lg border border-orange-200 bg-white p-1.5 shadow-lg dark:border-orange-500/30 dark:bg-gray-900"
      @keydown.esc.stop.prevent="emit('escape')"
    >
      <slot />
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'

const props = defineProps<{ id: string; anchor: HTMLElement | null }>()
const emit = defineEmits<{ close: []; escape: [] }>()
const panel = ref<HTMLElement | null>(null)
const viewportPadding = 8
const gap = 4
const maxPanelHeight = 240
let frame = 0

function positionPanel() {
  const anchor = props.anchor
  const element = panel.value
  if (!anchor || !element) return
  const bounds = anchor.getBoundingClientRect()
  const viewport = document.documentElement
  const scrollBody = anchor.closest('.patch-scroll-body')?.getBoundingClientRect()
  if (
    viewport.clientHeight &&
    (!bounds.height ||
      (scrollBody && (bounds.bottom <= scrollBody.top || bounds.top >= scrollBody.bottom)))
  ) {
    emit('close')
    return
  }

  const width = Math.min(bounds.width, viewport.clientWidth - 2 * viewportPadding)
  const left = Math.max(
    viewportPadding,
    Math.min(bounds.left, viewport.clientWidth - width - viewportPadding),
  )
  const widthStyle = `${Math.max(0, width)}px`
  if (element.style.width !== widthStyle) element.style.width = widthStyle
  const wantedHeight = Math.min(maxPanelHeight, element.scrollHeight + 2)
  const below = viewport.clientHeight - viewportPadding - bounds.bottom - gap
  const above = bounds.top - gap - viewportPadding
  const openAbove = below < wantedHeight && above > below
  const height = Math.max(0, Math.min(maxPanelHeight, openAbove ? above : below))
  const position = {
    left: `${left}px`,
    top: openAbove ? 'auto' : `${bounds.bottom + gap}px`,
    bottom: openAbove ? `${viewport.clientHeight - bounds.top + gap}px` : 'auto',
    maxHeight: `${height}px`,
  }
  for (const [key, value] of Object.entries(position) as [keyof typeof position, string][]) {
    if (element.style[key] !== value) element.style[key] = value
  }
  // Preview updates and scrolling can both move a row without resizing the anchor.
  frame = requestAnimationFrame(positionPanel)
}

function closeOutside(event: Event) {
  const target = event.target
  if (target instanceof Node && !props.anchor?.contains(target) && !panel.value?.contains(target)) {
    emit('close')
  }
}

onMounted(() => {
  document.addEventListener('pointerdown', closeOutside, true)
  document.addEventListener('focusin', closeOutside, true)
  positionPanel()
})

onUnmounted(() => {
  cancelAnimationFrame(frame)
  document.removeEventListener('pointerdown', closeOutside, true)
  document.removeEventListener('focusin', closeOutside, true)
})
</script>
