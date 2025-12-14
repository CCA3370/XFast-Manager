<template>
  <div class="app-container">
    <nav class="navbar bg-aviation-gray border-b border-aviation-blue">
      <div class="container mx-auto px-4 py-3 flex justify-between items-center">
        <h1 class="text-xl font-bold">XFastInstall</h1>
        <div class="nav-links space-x-4">
          <router-link to="/" class="hover:text-blue-400">Home</router-link>
          <router-link to="/settings" class="hover:text-blue-400">Settings</router-link>
        </div>
      </div>
    </nav>
    <main class="main-content">
      <router-view />
    </main>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { invoke } from '@tauri-apps/api/core'

const store = useAppStore()

onMounted(async () => {
  store.loadXplanePath()
  
  // Check if launched with CLI arguments
  try {
    const args = await invoke<string[]>('get_cli_args')
    if (args && args.length > 0) {
      console.log('Launched with arguments:', args)
      // TODO: Trigger analysis automatically
    }
  } catch (error) {
    console.error('Failed to get CLI args:', error)
  }
})
</script>

<style scoped>
.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.main-content {
  flex: 1;
  overflow-y: auto;
}
</style>
