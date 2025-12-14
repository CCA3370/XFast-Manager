<template>
  <div class="settings-view p-8">
    <div class="max-w-2xl mx-auto">
      <h2 class="text-3xl font-bold mb-6">Settings</h2>

      <div class="settings-card bg-aviation-gray rounded-lg p-6 mb-6">
        <h3 class="text-xl font-semibold mb-4">X-Plane 12 Installation Path</h3>
        <div class="flex gap-4">
          <input
            v-model="xplanePathInput"
            type="text"
            placeholder="/path/to/X-Plane 12"
            class="flex-1 px-4 py-2 bg-gray-800 border border-gray-600 rounded focus:outline-none focus:border-blue-500"
          />
          <button
            @click="selectFolder"
            class="px-6 py-2 bg-blue-600 hover:bg-blue-700 rounded transition"
          >
            Browse
          </button>
        </div>
        <button
          @click="savePath"
          class="mt-4 px-6 py-2 bg-green-600 hover:bg-green-700 rounded transition"
        >
          Save
        </button>
        <p v-if="saveMessage" class="mt-2 text-sm" :class="saveMessage.includes('Error') ? 'text-red-400' : 'text-green-400'">
          {{ saveMessage }}
        </p>
      </div>

      <div class="settings-card bg-aviation-gray rounded-lg p-6 mb-6" v-if="isWindows">
        <h3 class="text-xl font-semibold mb-4">Windows Integration</h3>
        <p class="mb-4 text-sm opacity-75">Add "Install to X-Plane" to the Windows context menu (right-click menu)</p>
        <button
          @click="registerContextMenu"
          class="px-6 py-2 bg-blue-600 hover:bg-blue-700 rounded transition mr-4"
        >
          Register Context Menu
        </button>
        <button
          @click="unregisterContextMenu"
          class="px-6 py-2 bg-red-600 hover:bg-red-700 rounded transition"
        >
          Unregister
        </button>
        <p v-if="registryMessage" class="mt-2 text-sm text-blue-400">
          {{ registryMessage }}
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

const store = useAppStore()
const xplanePathInput = ref('')
const saveMessage = ref('')
const registryMessage = ref('')
const isWindows = ref(false)

onMounted(async () => {
  xplanePathInput.value = store.xplanePath
  
  try {
    const platform = await invoke<string>('get_platform')
    isWindows.value = platform === 'windows'
  } catch (error) {
    console.error('Failed to get platform:', error)
  }
})

async function selectFolder() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select X-Plane 12 Folder'
    })
    
    if (selected) {
      xplanePathInput.value = selected as string
    }
  } catch (error) {
    console.error('Failed to open folder dialog:', error)
  }
}

function savePath() {
  if (!xplanePathInput.value) {
    saveMessage.value = 'Error: Please enter a path'
    return
  }

  store.setXplanePath(xplanePathInput.value)
  saveMessage.value = '✓ Path saved successfully'
  setTimeout(() => {
    saveMessage.value = ''
  }, 3000)
}

async function registerContextMenu() {
  try {
    await invoke('register_context_menu')
    registryMessage.value = '✓ Context menu registered successfully'
  } catch (error) {
    registryMessage.value = 'Error: ' + error
  }
}

async function unregisterContextMenu() {
  try {
    await invoke('unregister_context_menu')
    registryMessage.value = '✓ Context menu unregistered successfully'
  } catch (error) {
    registryMessage.value = 'Error: ' + error
  }
}
</script>

<style scoped>
.settings-card {
  border: 1px solid rgba(59, 130, 246, 0.2);
}
</style>
