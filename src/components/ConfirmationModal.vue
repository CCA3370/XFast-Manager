<template>
  <Teleport to="body">
    <transition name="modal">
      <div class="modal-overlay animate-fade-in" @click.self.stop>
        <div class="modal-content animate-scale-in" @click.stop>
          <!-- Header -->
          <div class="modal-header mb-2 flex-shrink-0">
            <div class="flex items-center justify-between">
              <div class="flex items-center space-x-2">
                <div
                  class="w-9 h-9 bg-blue-600 dark:bg-blue-500 rounded-lg flex items-center justify-center"
                >
                  <svg
                    class="w-4.5 h-4.5 text-white"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                    ></path>
                  </svg>
                </div>
                <div>
                  <h3 class="text-base font-bold text-gray-900 dark:text-white">
                    <AnimatedText>{{ $t('modal.confirmInstallation') }}</AnimatedText>
                  </h3>
                  <p class="text-gray-500 dark:text-gray-400 text-xs leading-tight">
                    <AnimatedText>{{ $t('modal.installToXplane') }}</AnimatedText>
                  </p>
                </div>
              </div>
              <div
                class="flex items-center space-x-1.5 px-2.5 py-1 bg-blue-50 dark:bg-blue-500/10 border border-blue-200 dark:border-blue-500/20 rounded-lg"
              >
                <svg
                  class="w-3.5 h-3.5 text-blue-600 dark:text-blue-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
                  ></path>
                </svg>
                <span class="text-xs font-semibold text-blue-700 dark:text-blue-300">
                  <AnimatedText>{{ store.enabledTasksCount }}/{{ store.currentTasks.length }}</AnimatedText>
                </span>
              </div>
            </div>
          </div>

          <!-- Size Warning Banner (only show if any size warnings exist) -->
          <div
            v-if="store.hasSizeWarnings"
            class="mb-2 p-2 bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/20 rounded-lg flex-shrink-0"
          >
            <div class="flex items-start space-x-1.5">
              <svg
                class="w-3.5 h-3.5 text-red-500 dark:text-red-400 flex-shrink-0 mt-0.5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
                ></path>
              </svg>
              <div class="flex-1">
                <span class="font-medium text-xs text-red-700 dark:text-red-100"><AnimatedText>{{ $t('modal.sizeWarningTitle') }}</AnimatedText></span>
                <p class="text-xs text-red-600 dark:text-red-200/70 leading-tight">
                  <AnimatedText>{{ $t('modal.sizeWarningDesc') }}</AnimatedText>
                </p>
              </div>
            </div>
          </div>

          <!-- Tasks List -->
          <TaskListSection />

          <!-- Actions -->
          <div class="flex justify-end gap-1.5 flex-shrink-0 pt-1.5">
            <button
              class="px-3 py-1.5 bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-700 text-gray-700 dark:text-white rounded-lg transition-all duration-200 hover:scale-105 text-xs font-medium flex items-center space-x-1"
              @click="$emit('close')"
            >
              <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M6 18L18 6M6 6l12 12"
                ></path>
              </svg>
              <span><AnimatedText>{{ $t('common.cancel') }}</AnimatedText></span>
            </button>
            <button
              :disabled="installDisabled"
              :class="[
                'px-3 py-1.5 rounded-lg transition-all duration-200 text-xs font-medium flex items-center space-x-1',
                installDisabled
                  ? 'bg-gray-300 dark:bg-gray-600 cursor-not-allowed opacity-50 text-gray-500 dark:text-gray-400'
                  : 'bg-gradient-to-r from-green-600 to-emerald-600 hover:from-green-700 hover:to-emerald-700 hover:scale-105 text-white',
              ]"
              @click="$emit('confirm')"
            >
              <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M5 13l4 4L19 7"
                ></path>
              </svg>
              <span><AnimatedText>{{ $t('modal.startInstallation') }}</AnimatedText></span>
            </button>
          </div>
        </div>
      </div>
    </transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'
import AnimatedText from '@/components/AnimatedText.vue'
import TaskListSection from '@/components/confirmation/TaskListSection.vue'

const store = useAppStore()

defineEmits(['close', 'confirm'])

// Check if install button should be disabled
const installDisabled = computed(() => {
  // Disable if no tasks are enabled
  if (store.enabledTasksCount === 0) return true
  // Disable if there are size warnings that haven't been confirmed
  return store.hasSizeWarnings && !store.allSizeWarningsConfirmed
})
</script>

<style scoped>
/* Modal animations */
.modal-enter-active,
.modal-leave-active {
  transition: all 0.3s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal-content,
.modal-leave-to .modal-content {
  opacity: 0;
  transform: scale(0.9) translateY(-20px);
}

@keyframes scale-in {
  from {
    opacity: 0;
    transform: scale(0.9) translateY(-20px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.animate-scale-in {
  animation: scale-in 0.4s ease-out;
}

.animate-fade-in {
  animation: fade-in 0.3s ease-out;
}

@keyframes fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

/* Modal overlay */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.8);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

/* Modal content */
.modal-content {
  background: white;
  border-radius: 0.75rem;
  padding: 0.875rem;
  max-width: 520px;
  width: 90%;
  border: 1px solid rgba(229, 231, 235, 1);
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
  max-height: 88vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.dark .modal-content {
  background: linear-gradient(135deg, rgba(31, 41, 55, 0.95), rgba(17, 24, 39, 0.95));
  border: 1px solid rgba(59, 130, 246, 0.3);
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.8);
}
</style>
