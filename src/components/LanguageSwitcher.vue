<template>
  <div ref="rootRef" class="relative">
    <button
      type="button"
      class="flex h-8 w-[100px] items-center justify-between gap-2 rounded-xl border border-white/20 bg-white/10 px-2.5 text-[13px] text-gray-700 backdrop-blur-md transition-all duration-300 hover:bg-white/20 focus:outline-none focus:ring-2 focus:ring-blue-500/40 dark:text-gray-100 dark:hover:bg-white/15"
      :class="isOpen ? 'bg-white/20 shadow-sm dark:bg-white/15' : ''"
      :title="`${$t('common.language')}: ${currentOption.label}`"
      :aria-expanded="isOpen"
      :aria-label="$t('common.language')"
      @click="toggleMenu"
    >
      <span class="min-w-0 truncate font-medium">{{ currentOption.label }}</span>
      <svg
        class="h-3.5 w-3.5 flex-none text-gray-500 transition-transform duration-200 dark:text-gray-300"
        :class="isOpen ? 'rotate-180' : ''"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
      </svg>
    </button>

    <Transition name="language-menu">
      <div
        v-if="isOpen"
        class="absolute right-0 top-full z-50 mt-2 w-52 overflow-hidden rounded-2xl border border-gray-200/70 bg-white/92 p-1.5 shadow-[0_18px_50px_-18px_rgba(15,23,42,0.45)] backdrop-blur-xl dark:border-white/10 dark:bg-gray-900/92"
      >
        <button
          v-for="option in LOCALE_OPTIONS"
          :key="option.value"
          type="button"
          class="block w-full rounded-xl px-3 py-2.5 text-left text-sm font-medium transition-all duration-200"
          :class="
            currentLanguage === option.value
              ? 'bg-blue-50 text-blue-700 shadow-sm dark:bg-blue-500/15 dark:text-blue-200'
              : 'text-gray-700 hover:bg-gray-100/90 dark:text-gray-100 dark:hover:bg-white/8'
          "
          @click="selectLanguage(option.value)"
        >
          {{ option.label }}
        </button>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { setLocale } from '@/i18n'
import { LOCALE_OPTIONS, type SupportedLocale } from '@/i18n/shared'

const { locale } = useI18n()

const rootRef = ref<HTMLElement | null>(null)
const isOpen = ref(false)

const currentLanguage = computed(() => locale.value as SupportedLocale)
const currentOption = computed(
  () =>
    LOCALE_OPTIONS.find((option) => option.value === currentLanguage.value) ?? LOCALE_OPTIONS[0],
)

function closeMenu() {
  isOpen.value = false
}

function toggleMenu() {
  isOpen.value = !isOpen.value
}

async function selectLanguage(value: SupportedLocale) {
  closeMenu()

  if (value === currentLanguage.value) {
    return
  }

  await setLocale(value)
}

function handlePointerDown(event: MouseEvent) {
  if (!rootRef.value?.contains(event.target as Node)) {
    closeMenu()
  }
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    closeMenu()
  }
}

onMounted(() => {
  document.addEventListener('mousedown', handlePointerDown)
  document.addEventListener('keydown', handleKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', handlePointerDown)
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<style scoped>
.language-menu-enter-active,
.language-menu-leave-active {
  transition:
    opacity 0.18s ease,
    transform 0.18s ease;
}

.language-menu-enter-from,
.language-menu-leave-to {
  opacity: 0;
  transform: translateY(-6px) scale(0.98);
}
</style>
