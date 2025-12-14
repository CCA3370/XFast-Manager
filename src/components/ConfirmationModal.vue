<template>
  <div class="modal-overlay" @click="$emit('close')">
    <div class="modal-content" @click.stop>
      <h3 class="text-2xl font-bold mb-4">Confirm Installation</h3>
      
      <div class="tasks-list mb-6 max-h-96 overflow-y-auto">
        <div
          v-for="task in store.currentTasks"
          :key="task.id"
          class="task-item bg-gray-800 rounded p-4 mb-3"
        >
          <div class="flex items-start justify-between">
            <div>
              <div class="flex items-center gap-2 mb-2">
                <span class="type-badge" :class="getTypeBadgeClass(task.type)">
                  {{ task.type }}
                </span>
                <span class="font-semibold">{{ task.displayName }}</span>
              </div>
              <p class="text-sm opacity-75">→ {{ task.targetPath }}</p>
              <p v-if="task.conflictExists" class="text-sm text-yellow-400 mt-2">
                ⚠️ Target folder already exists
              </p>
            </div>
          </div>
        </div>
      </div>

      <div class="flex justify-end gap-4">
        <button
          @click="$emit('close')"
          class="px-6 py-2 bg-gray-600 hover:bg-gray-700 rounded transition"
        >
          Cancel
        </button>
        <button
          @click="$emit('confirm')"
          class="px-6 py-2 bg-green-600 hover:bg-green-700 rounded transition"
        >
          Install
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useAppStore } from '@/stores/app'
import { AddonType } from '@/types'

const store = useAppStore()

defineEmits(['close', 'confirm'])

function getTypeBadgeClass(type: AddonType) {
  switch (type) {
    case AddonType.Aircraft:
      return 'bg-blue-600'
    case AddonType.Scenery:
      return 'bg-green-600'
    case AddonType.Plugin:
      return 'bg-purple-600'
    case AddonType.Navdata:
      return 'bg-orange-600'
    default:
      return 'bg-gray-600'
  }
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.8);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: #1f2937;
  border-radius: 1rem;
  padding: 2rem;
  max-width: 600px;
  width: 90%;
  border: 1px solid rgba(59, 130, 246, 0.3);
}

.type-badge {
  display: inline-block;
  padding: 0.25rem 0.75rem;
  border-radius: 0.375rem;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
}
</style>
