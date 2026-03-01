import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { AddonUpdateDrawerTask } from '@/types'

function taskKey(task: Pick<AddonUpdateDrawerTask, 'itemType' | 'folderName'>): string {
  return `${task.itemType}:${task.folderName}`
}

export const useAddonUpdateDrawerStore = defineStore('addonUpdateDrawer', () => {
  const show = ref(false)
  const tasks = ref<AddonUpdateDrawerTask[]>([])
  const activeTaskKey = ref('')

  function openTasks(newTasks: AddonUpdateDrawerTask[], focusKey?: string) {
    if (!newTasks.length) {
      return
    }

    const merged = new Map(tasks.value.map((task) => [taskKey(task), task]))
    for (const task of newTasks) {
      merged.set(taskKey(task), task)
    }

    tasks.value = Array.from(merged.values())
    activeTaskKey.value = focusKey || taskKey(newTasks[0])
    show.value = true
  }

  function openTask(task: AddonUpdateDrawerTask) {
    openTasks([task], taskKey(task))
  }

  function selectTask(key: string) {
    if (!key) return
    activeTaskKey.value = key
  }

  function closeDrawer() {
    show.value = false
  }

  function clearTasks() {
    tasks.value = []
    activeTaskKey.value = ''
    show.value = false
  }

  return {
    show,
    tasks,
    activeTaskKey,
    openTask,
    openTasks,
    selectTask,
    closeDrawer,
    clearTasks,
  }
})
