<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useManagementStore } from '@/stores/management'
import { useAppStore } from '@/stores/app'
import { useToastStore } from '@/stores/toast'
import { useModalStore } from '@/stores/modal'
import LiveryCard from '@/components/LiveryCard.vue'
import type { LiveryInfo } from '@/types'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const managementStore = useManagementStore()
const appStore = useAppStore()
const toastStore = useToastStore()
const modalStore = useModalStore()

const aircraftFolder = computed(() => (route.query.aircraft as string) || '')
const liveries = ref<LiveryInfo[]>([])
const isLoading = ref(true)
const loadError = ref(false)
const previewSrc = ref<string | null>(null)
const previewName = ref('')
const previewScale = ref(1)
const previewX = ref(0)
const previewY = ref(0)
const isDragging = ref(false)
let dragStartX = 0
let dragStartY = 0
let dragStartPanX = 0
let dragStartPanY = 0

// Look up aircraft display name from management store
const aircraftDisplayName = computed(() => {
  const aircraft = managementStore.aircraft.find((a) => a.folderName === aircraftFolder.value)
  return aircraft?.displayName || aircraftFolder.value
})

const liveryCount = computed(() => liveries.value.length)

async function loadLiveries() {
  if (!appStore.xplanePath || !aircraftFolder.value) return

  isLoading.value = true
  loadError.value = false
  try {
    liveries.value = await invoke<LiveryInfo[]>('get_aircraft_liveries', {
      xplanePath: appStore.xplanePath,
      aircraftFolder: aircraftFolder.value,
    })
  } catch (e) {
    loadError.value = true
    modalStore.showError(t('livery.loadFailed') + ': ' + String(e))
  } finally {
    isLoading.value = false
  }
}

async function handleDeleteLivery(folderName: string) {
  if (!appStore.xplanePath) return

  try {
    await invoke('delete_aircraft_livery', {
      xplanePath: appStore.xplanePath,
      aircraftFolder: aircraftFolder.value,
      liveryFolder: folderName,
    })

    // Remove from local array
    liveries.value = liveries.value.filter((l) => l.folderName !== folderName)

    // Update livery count in management store
    const aircraft = managementStore.aircraft.find((a) => a.folderName === aircraftFolder.value)
    if (aircraft) {
      aircraft.liveryCount = liveries.value.length
      aircraft.hasLiveries = liveries.value.length > 0
    }

    toastStore.success(t('livery.deleteSuccess'))

    // Navigate back if no liveries remaining
    if (liveries.value.length === 0) {
      router.push('/management')
    }
  } catch (e) {
    modalStore.showError(t('livery.deleteFailed') + ': ' + String(e))
  }
}

function handlePreview(src: string, name: string) {
  previewSrc.value = src
  previewName.value = name
  previewScale.value = 1
  previewX.value = 0
  previewY.value = 0
}

function closePreview() {
  previewSrc.value = null
  previewScale.value = 1
  previewX.value = 0
  previewY.value = 0
  isDragging.value = false
}

function handleWheel(e: WheelEvent) {
  e.preventDefault()
  const delta = e.deltaY > 0 ? -0.1 : 0.1
  const newScale = Math.min(5, Math.max(0.5, previewScale.value + delta))
  previewScale.value = newScale
  if (newScale <= 1) {
    previewX.value = 0
    previewY.value = 0
  }
}

function handlePointerDown(e: PointerEvent) {
  if (previewScale.value <= 1) return
  isDragging.value = true
  dragStartX = e.clientX
  dragStartY = e.clientY
  dragStartPanX = previewX.value
  dragStartPanY = previewY.value
  ;(e.target as HTMLElement).setPointerCapture(e.pointerId)
}

function handlePointerMove(e: PointerEvent) {
  if (!isDragging.value) return
  previewX.value = dragStartPanX + (e.clientX - dragStartX)
  previewY.value = dragStartPanY + (e.clientY - dragStartY)
}

function handlePointerUp() {
  isDragging.value = false
}

watch(previewSrc, (val) => {
  const onKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') closePreview()
  }
  if (val) {
    window.addEventListener('keydown', onKeydown)
    window.addEventListener('wheel', handleWheel, { passive: false })
    const unwatch = watch(previewSrc, (newVal) => {
      if (!newVal) {
        window.removeEventListener('keydown', onKeydown)
        window.removeEventListener('wheel', handleWheel)
        unwatch()
      }
    })
  }
})

function goBack() {
  router.push('/management')
}

async function handleOpenLiveryFolder(folderName: string) {
  if (!appStore.xplanePath || !aircraftFolder.value) return
  try {
    await invoke('open_livery_folder', {
      xplanePath: appStore.xplanePath,
      aircraftFolder: aircraftFolder.value,
      liveryFolder: folderName,
    })
  } catch (e) {
    modalStore.showError(t('management.openFolderFailed') + ': ' + String(e))
  }
}

onMounted(() => {
  loadLiveries()
})
</script>

<template>
  <div class="liveries-view h-full flex flex-col p-4 overflow-hidden">
    <!-- Header -->
    <div class="mb-4 flex-shrink-0 flex items-center gap-3">
      <button
        class="p-1.5 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
        :title="t('onboarding.back')"
        @click="goBack"
      >
        <svg
          class="w-5 h-5 text-gray-600 dark:text-gray-300"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M15 19l-7-7 7-7"
          />
        </svg>
      </button>
      <div class="flex-1 min-w-0">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 truncate">
          {{ aircraftDisplayName }}
        </h2>
      </div>
      <span
        v-if="!isLoading"
        class="flex-shrink-0 px-2 py-0.5 rounded text-xs font-medium text-blue-700 dark:text-blue-300 bg-blue-100 dark:bg-blue-900/30"
      >
        {{ liveryCount }} {{ t('management.liveries') }}
      </span>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto">
      <!-- Loading state -->
      <div v-if="isLoading" class="flex items-center justify-center py-12">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
      </div>

      <!-- Error state -->
      <div v-else-if="loadError" class="text-center py-12">
        <p class="text-gray-600 dark:text-gray-400">{{ t('livery.loadFailed') }}</p>
      </div>

      <!-- Empty state -->
      <div v-else-if="liveries.length === 0" class="text-center py-12">
        <p class="text-gray-600 dark:text-gray-400">{{ t('livery.noLiveries') }}</p>
      </div>

      <!-- Livery grid -->
      <div v-else class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-4 pb-4">
        <LiveryCard
          v-for="livery in liveries"
          :key="livery.folderName"
          :livery="livery"
          @delete="handleDeleteLivery"
          @preview="(src: string) => handlePreview(src, livery.displayName)"
          @open-folder="handleOpenLiveryFolder"
        />
      </div>
    </div>

    <!-- Lightbox overlay -->
    <Teleport to="body">
      <Transition
        enter-active-class="transition duration-200 ease-out"
        enter-from-class="opacity-0"
        enter-to-class="opacity-100"
        leave-active-class="transition duration-150 ease-in"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <div
          v-if="previewSrc"
          class="fixed inset-0 z-[100] flex flex-col items-center justify-center bg-black/80 backdrop-blur-sm"
          @click.self="closePreview"
        >
          <button
            class="absolute top-4 right-4 z-10 w-10 h-10 flex items-center justify-center rounded-full bg-white/15 hover:bg-white/30 text-white text-xl transition-colors"
            @click="closePreview"
          >
            ✕
          </button>
          <img
            :src="previewSrc"
            :alt="previewName"
            class="max-w-[90vw] max-h-[85vh] object-contain rounded-lg shadow-2xl select-none"
            :class="previewScale > 1 ? 'cursor-grab' : ''"
            :style="{
              transform: `translate(${previewX}px, ${previewY}px) scale(${previewScale})`,
              transition: isDragging ? 'none' : 'transform 0.1s',
            }"
            draggable="false"
            @pointerdown="handlePointerDown"
            @pointermove="handlePointerMove"
            @pointerup="handlePointerUp"
          />
          <p class="mt-3 text-sm text-white/80 text-center truncate max-w-[90vw]">
            {{ previewName }}
          </p>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.liveries-view {
  background: linear-gradient(to bottom, rgba(248, 250, 252, 0.5), rgba(241, 245, 249, 0.5));
}

.dark .liveries-view {
  background: linear-gradient(to bottom, rgba(17, 24, 39, 0.5), rgba(31, 41, 55, 0.5));
}
</style>
