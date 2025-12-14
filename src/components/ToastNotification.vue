<template>
  <Teleport to="body">
    <div class="toast-container">
      <transition-group name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="toast"
          :class="getToastClass(toast.type)"
        >
          <span>{{ toast.message }}</span>
          <button @click="removeToast(toast.id)" class="toast-close">×</button>
        </div>
      </transition-group>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useToastStore } from '@/stores/toast'

const toastStore = useToastStore()
const toasts = computed(() => toastStore.toasts)

function getToastClass(type: string) {
  switch (type) {
    case 'error':
      return 'toast-error'
    case 'success':
      return 'toast-success'
    case 'warning':
      return 'toast-warning'
    default:
      return 'toast-info'
  }
}

function removeToast(id: string) {
  toastStore.remove(id)
}
</script>

<style scoped>
.toast-container {
  position: fixed;
  top: 1rem;
  right: 1rem;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  max-width: 400px;
}

.toast {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.5rem;
  border-radius: 0.5rem;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
  min-width: 300px;
  backdrop-filter: blur(10px);
}

.toast-info {
  background: rgba(59, 130, 246, 0.9);
  color: white;
}

.toast-success {
  background: rgba(34, 197, 94, 0.9);
  color: white;
}

.toast-error {
  background: rgba(239, 68, 68, 0.9);
  color: white;
}

.toast-warning {
  background: rgba(251, 191, 36, 0.9);
  color: white;
}

.toast-close {
  background: transparent;
  border: none;
  color: white;
  font-size: 1.5rem;
  cursor: pointer;
  padding: 0;
  margin-left: 1rem;
  line-height: 1;
  opacity: 0.7;
  transition: opacity 0.2s;
}

.toast-close:hover {
  opacity: 1;
}

.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(100%);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(100%);
}
</style>
