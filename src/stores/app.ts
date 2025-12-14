import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { InstallTask } from '@/types'

export const useAppStore = defineStore('app', () => {
  const xplanePath = ref<string>('')
  const currentTasks = ref<InstallTask[]>([])
  const isAnalyzing = ref(false)
  const isInstalling = ref(false)

  function setXplanePath(path: string) {
    xplanePath.value = path
    localStorage.setItem('xplanePath', path)
  }

  function loadXplanePath() {
    const saved = localStorage.getItem('xplanePath')
    if (saved) {
      xplanePath.value = saved
    }
  }

  function setCurrentTasks(tasks: InstallTask[]) {
    currentTasks.value = tasks
  }

  function clearTasks() {
    currentTasks.value = []
  }

  return {
    xplanePath,
    currentTasks,
    isAnalyzing,
    isInstalling,
    setXplanePath,
    loadXplanePath,
    setCurrentTasks,
    clearTasks,
  }
})
