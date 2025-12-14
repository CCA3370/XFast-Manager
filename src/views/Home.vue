<template>
  <div class="home-view p-8">
    <div class="max-w-4xl mx-auto">
      <h2 class="text-3xl font-bold mb-6">X-Plane 12 Addon Installer</h2>
      
      <div 
        v-if="!store.xplanePath"
        class="alert bg-yellow-900/50 border border-yellow-700 rounded p-4 mb-6"
      >
        <p>⚠️ Please set your X-Plane 12 path in <router-link to="/settings" class="underline">Settings</router-link> first.</p>
      </div>

      <div
        class="drop-zone"
        :class="{ 'drag-over': isDragging }"
        @drop="handleDrop"
        @dragover.prevent="isDragging = true"
        @dragleave="isDragging = false"
        @dragenter.prevent
      >
        <div class="drop-zone-content">
          <svg class="w-24 h-24 mx-auto mb-4 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path>
          </svg>
          <p class="text-xl mb-2">Drop your addon files here</p>
          <p class="text-sm opacity-75">Supports .zip, .7z, .rar files, or folders</p>
        </div>
      </div>

      <div v-if="store.isAnalyzing" class="mt-8 text-center">
        <p class="text-lg">🔍 Analyzing files...</p>
      </div>

      <ConfirmationModal v-if="showConfirmation" @close="showConfirmation = false" @confirm="handleInstall" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '@/stores/app'
import { invoke } from '@tauri-apps/api/core'
import ConfirmationModal from '@/components/ConfirmationModal.vue'
import type { AnalysisResult } from '@/types'

const store = useAppStore()
const isDragging = ref(false)
const showConfirmation = ref(false)

async function handleDrop(event: DragEvent) {
  event.preventDefault()
  isDragging.value = false

  if (!store.xplanePath) {
    alert('Please set X-Plane path in Settings first')
    return
  }

  const files = event.dataTransfer?.files
  if (!files || files.length === 0) return

  store.isAnalyzing = true

  try {
    const paths = Array.from(files).map(file => file.path)
    const result = await invoke<AnalysisResult>('analyze_addons', {
      paths,
      xplanePath: store.xplanePath
    })

    if (result.errors.length > 0) {
      alert('Errors during analysis:\n' + result.errors.join('\n'))
    }

    if (result.tasks.length > 0) {
      store.setCurrentTasks(result.tasks)
      showConfirmation.value = true
    } else {
      alert('No valid addons detected')
    }
  } catch (error) {
    console.error('Analysis failed:', error)
    alert('Failed to analyze addons: ' + error)
  } finally {
    store.isAnalyzing = false
  }
}

async function handleInstall() {
  showConfirmation.value = false
  store.isInstalling = true

  try {
    await invoke('install_addons', {
      tasks: store.currentTasks
    })
    alert('Installation completed successfully!')
    store.clearTasks()
  } catch (error) {
    console.error('Installation failed:', error)
    alert('Installation failed: ' + error)
  } finally {
    store.isInstalling = false
  }
}
</script>

<style scoped>
.drop-zone {
  border: 3px dashed #4b5563;
  border-radius: 1rem;
  padding: 4rem 2rem;
  text-align: center;
  transition: all 0.3s ease;
  background: rgba(31, 41, 55, 0.5);
}

.drop-zone:hover {
  border-color: #1e3a8a;
  background: rgba(31, 41, 55, 0.7);
}

.drop-zone.drag-over {
  border-color: #3b82f6;
  background: rgba(59, 130, 246, 0.1);
  transform: scale(1.02);
}
</style>
