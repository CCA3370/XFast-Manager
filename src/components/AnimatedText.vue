<template>
  <transition name="text-fade" mode="out-in" appear>
    <span :key="key">
      <slot></slot>
    </span>
  </transition>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { locale } = useI18n()

// 使用locale作为key来触发重新渲染
const key = computed(() => locale.value)
</script>

<style scoped>
.text-fade-enter-active,
.text-fade-leave-active {
  transition: all 0.25s ease-out;
}

.text-fade-enter-from {
  opacity: 0;
  transform: translateY(-4px) scale(0.98);
}

.text-fade-leave-to {
  opacity: 0;
  transform: translateY(4px) scale(0.98);
}

.text-fade-enter-to,
.text-fade-leave-from {
  opacity: 1;
  transform: translateY(0) scale(1);
}
</style>
