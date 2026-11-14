<template>
  <div class="flex min-h-full items-center justify-center overflow-y-auto p-5">
    <section
      class="w-full max-w-xl space-y-6 rounded-xl border border-gray-200 bg-white/80 p-6 shadow-sm dark:border-white/10 dark:bg-gray-800/40"
      aria-labelledby="onboarding-title"
    >
      <h1 id="onboarding-title" class="text-xl font-semibold">{{ t('onboarding.title') }}</h1>
      <XplaneDirectoryForm onboarding @saved="finish" />
    </section>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import XplaneDirectoryForm from '@/components/XplaneDirectoryForm.vue'

const { t } = useI18n()
const router = useRouter()

async function finish() {
  await router.replace('/')
}
</script>
