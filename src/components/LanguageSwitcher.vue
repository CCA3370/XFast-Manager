<template>
  <button
    @click="toggleLanguage"
    class="flex items-center justify-center w-9 h-9 text-sm bg-white/10 hover:bg-white/20 rounded-lg transition-all duration-300 hover:scale-105 backdrop-blur-sm border border-white/20"
    :title="$t('common.language')"
  >
    <transition name="language-fade" mode="out-in">
      <span :key="currentLanguage" class="font-medium text-center">
        {{ currentLanguage.toUpperCase() }}
      </span>
    </transition>
  </button>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { setLocale } from '@/i18n'

const { locale } = useI18n()

const currentLanguage = computed(() => locale.value)

const toggleLanguage = () => {
  const newLocale = locale.value === 'zh' ? 'en' : 'zh'
  setLocale(newLocale)
}
</script>

<style scoped>
.language-fade-enter-active,
.language-fade-leave-active {
  transition: all 0.3s ease;
}

.language-fade-enter-from {
  opacity: 0;
  transform: translateY(-10px);
}

.language-fade-leave-to {
  opacity: 0;
  transform: translateY(10px);
}
</style>