<template>
  <div class="relative">
    <select
      v-model="currentLanguage"
      class="h-8 w-[118px] appearance-none rounded-lg border border-white/20 bg-white/10 py-0 pl-3 pr-8 text-sm text-gray-700 backdrop-blur-sm transition-all duration-300 hover:bg-white/20 focus:outline-none focus:ring-2 focus:ring-blue-500/40 dark:text-gray-100"
      :title="$t('common.language')"
      @change="handleChange"
    >
      <option
        v-for="option in LOCALE_OPTIONS"
        :key="option.value"
        :value="option.value"
        class="bg-white text-gray-900 dark:bg-gray-900 dark:text-gray-100"
      >
        {{ option.label }}
      </option>
    </select>
    <svg
      class="pointer-events-none absolute right-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-gray-500 dark:text-gray-300"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { setLocale } from '@/i18n'
import { LOCALE_OPTIONS, type SupportedLocale } from '@/i18n/shared'

const { locale } = useI18n()

const currentLanguage = computed({
  get: () => locale.value as SupportedLocale,
  set: (value: SupportedLocale) => {
    locale.value = value
  },
})

const handleChange = async (event: Event) => {
  const target = event.target as HTMLSelectElement
  await setLocale(target.value as SupportedLocale)
}
</script>
