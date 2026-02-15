<template>
  <Teleport to="body">
    <Transition
      :css="false"
      @enter="onEnter"
      @leave="onLeave"
    >
      <div
        v-if="show"
        class="fixed inset-0 z-[1100] flex items-center justify-center"
      >
        <!-- Backdrop -->
        <div
          ref="backdropRef"
          class="absolute inset-0 bg-black/50 backdrop-blur-sm"
          @click="$emit('close')"
        />

        <!-- Card -->
        <div
          ref="cardRef"
          class="relative bg-white dark:bg-gradient-to-tr dark:from-gray-800/95 dark:to-gray-900/95 rounded-2xl max-w-sm w-full p-6 mx-4 border border-gray-200 dark:border-gray-700/50 shadow-2xl"
          @click.stop
        >
          <!-- Header -->
          <div class="flex items-center justify-between mb-5">
            <div class="flex items-center space-x-2.5">
              <div class="w-8 h-8 bg-pink-500 rounded-lg flex items-center justify-center">
                <svg class="w-4.5 h-4.5 text-white" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z" />
                </svg>
              </div>
              <div>
                <h3 class="text-base font-bold text-gray-900 dark:text-white">{{ $t('sponsor.title') }}</h3>
                <p class="text-pink-500 dark:text-pink-400/80 text-xs mt-0.5">{{ $t('sponsor.subtitle') }}</p>
              </div>
            </div>
            <button
              @click="$emit('close')"
              class="p-1.5 rounded-lg text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-white/10 transition-colors"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- Tab Switcher -->
          <div class="flex bg-gray-100 dark:bg-white/5 rounded-xl p-1 mb-5">
            <button
              @click="activeTab = 'wechat'"
              class="flex-1 flex items-center justify-center space-x-1.5 py-2 px-3 rounded-lg text-xs font-medium transition-all duration-200"
              :class="activeTab === 'wechat'
                ? 'bg-white dark:bg-white/10 text-gray-900 dark:text-white shadow-sm'
                : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'"
            >
              <!-- WeChat icon -->
              <svg class="w-4 h-4 transition-colors duration-200" viewBox="0 0 24 24" :fill="activeTab === 'wechat' ? '#07C160' : 'currentColor'">
                <path d="M8.691 2.188C3.891 2.188 0 5.476 0 9.53c0 2.212 1.17 4.203 3.002 5.55a.59.59 0 01.213.665l-.39 1.48c-.019.07-.048.141-.048.213 0 .163.13.295.29.295a.326.326 0 00.167-.054l1.903-1.114a.864.864 0 01.717-.098 10.16 10.16 0 002.837.403c.276 0 .543-.027.811-.05-.857-2.578.157-4.972 1.932-6.446 1.703-1.415 3.882-1.98 5.853-1.838-.576-3.583-4.196-6.348-8.596-6.348zM5.785 5.991c.642 0 1.162.529 1.162 1.18a1.17 1.17 0 01-1.162 1.178A1.17 1.17 0 014.623 7.17c0-.651.52-1.18 1.162-1.18zm5.813 0c.642 0 1.162.529 1.162 1.18a1.17 1.17 0 01-1.162 1.178 1.17 1.17 0 01-1.162-1.178c0-.651.52-1.18 1.162-1.18zm5.34 2.867c-1.797-.052-3.746.512-5.28 1.786-1.72 1.428-2.687 3.72-1.78 6.22.942 2.453 3.666 4.229 6.884 4.229.826 0 1.622-.12 2.361-.336a.722.722 0 01.598.082l1.584.926a.272.272 0 00.14.047c.134 0 .24-.111.24-.247 0-.06-.023-.12-.038-.177l-.327-1.233a.582.582 0 01-.023-.156.49.49 0 01.201-.398C23.024 18.48 24 16.82 24 14.98c0-3.21-2.931-5.837-7.062-6.122zm-2.036 2.87c.535 0 .969.44.969.982a.976.976 0 01-.969.983.976.976 0 01-.969-.983c0-.542.434-.983.97-.983zm4.842 0c.535 0 .969.44.969.982a.976.976 0 01-.969.983.976.976 0 01-.969-.983c0-.542.434-.983.969-.983z"/>
              </svg>
              <span>{{ $t('sponsor.wechat') }}</span>
            </button>
            <button
              @click="activeTab = 'alipay'"
              class="flex-1 flex items-center justify-center space-x-1.5 py-2 px-3 rounded-lg text-xs font-medium transition-all duration-200"
              :class="activeTab === 'alipay'
                ? 'bg-white dark:bg-white/10 text-gray-900 dark:text-white shadow-sm'
                : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'"
            >
              <!-- Alipay icon -->
              <svg class="w-4 h-4 transition-colors duration-200" viewBox="0 0 1024.051 1024" :fill="activeTab === 'alipay' ? '#009FE8' : 'currentColor'">
                <path d="m1024.051 701.03v-504.166a196.966 196.966 0 0 0 -196.915-196.864h-630.272a196.966 196.966 0 0 0 -196.864 196.864v630.272a196.915 196.915 0 0 0 196.864 196.864h630.272a197.12 197.12 0 0 0 193.843-162.1c-52.224-22.63-278.528-120.32-396.441-176.64-89.703 108.698-183.706 173.927-325.325 173.927s-236.186-87.245-224.82-194.047c7.476-70.041 55.553-184.576 264.295-164.966 110.08 10.342 160.41 30.873 250.163 60.518 23.194-42.598 42.496-89.446 57.14-139.264h-397.928v-39.424h196.915v-70.86h-240.178v-43.367h240.128v-102.145s2.15-15.974 19.814-15.974h98.458v118.118h256v43.418h-256v70.758h208.845a805.99 805.99 0 0 1 -84.839 212.685c60.672 22.016 336.794 106.393 336.794 106.393zm-740.505 90.573c-149.658 0-173.312-94.464-165.376-133.939 7.833-39.322 51.2-90.624 134.4-90.624 95.59 0 181.248 24.474 284.057 74.547-72.192 94.003-160.921 150.016-253.081 150.016z"/>
              </svg>
              <span>{{ $t('sponsor.alipay') }}</span>
            </button>
          </div>

          <!-- QR Code -->
          <div class="flex flex-col items-center">
            <div class="w-52 h-52 bg-white rounded-xl p-2 shadow-inner border border-gray-100 dark:border-gray-600/30 overflow-hidden">
              <Transition name="qr-switch" mode="out-in">
                <img
                  :key="activeTab"
                  :src="activeTab === 'wechat' ? wechatQR : alipayQR"
                  :alt="activeTab === 'wechat' ? 'WeChat Pay' : 'Alipay'"
                  class="w-full h-full object-contain rounded-lg"
                />
              </Transition>
            </div>
            <p class="mt-3 text-xs text-gray-500 dark:text-gray-400">
              {{ $t('sponsor.scanTip') }}
            </p>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import wechatQR from '@/assets/sponsor/wechat.png'
import alipayQR from '@/assets/sponsor/alipay.png'

defineProps<{ show: boolean }>()
defineEmits<{ close: [] }>()

const activeTab = ref<'wechat' | 'alipay'>('wechat')
const backdropRef = ref<HTMLElement | null>(null)
const cardRef = ref<HTMLElement | null>(null)

function onEnter(el: Element, done: () => void) {
  const container = el as HTMLElement
  const backdrop = backdropRef.value
  const card = cardRef.value
  if (!backdrop || !card) { done(); return }

  // Initial state
  container.style.opacity = '0'
  backdrop.style.opacity = '0'
  card.style.opacity = '0'
  card.style.transform = 'scale(0.85) translateY(-30px)'

  // Force reflow
  container.offsetHeight

  // Set transitions
  container.style.transition = 'opacity 0.15s ease-out'
  backdrop.style.transition = 'opacity 0.15s ease-out'
  card.style.transition = 'all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1)'

  requestAnimationFrame(() => {
    container.style.opacity = '1'
    backdrop.style.opacity = '1'
    setTimeout(() => {
      card.style.opacity = '1'
      card.style.transform = 'scale(1) translateY(0)'
    }, 50)
  })

  setTimeout(done, 450)
}

function onLeave(el: Element, done: () => void) {
  const container = el as HTMLElement
  const backdrop = container.querySelector('.bg-black\\/50') as HTMLElement
  const card = container.querySelector('.rounded-2xl') as HTMLElement
  if (!backdrop || !card) { done(); return }

  container.style.transition = 'opacity 0.3s ease-in'
  backdrop.style.transition = 'opacity 0.3s ease-in'
  card.style.transition = 'all 0.3s cubic-bezier(0.4, 0, 0.6, 1)'

  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      container.style.opacity = '0'
      backdrop.style.opacity = '0'
      card.style.opacity = '0'
      card.style.transform = 'scale(0.9) translateY(10px)'
    })
  })

  setTimeout(done, 350)
}
</script>

<style scoped>
.qr-switch-enter-active,
.qr-switch-leave-active {
  transition: all 0.2s ease;
}

.qr-switch-enter-from {
  opacity: 0;
  transform: scale(0.9);
}

.qr-switch-leave-to {
  opacity: 0;
  transform: scale(0.9);
}
</style>
