<template>
  <Teleport to="body">
    <transition name="modal">
      <div class="modal-overlay" @click="handleCancel">
        <div class="modal-content animate-scale-in" @click.stop>
          <!-- Header -->
          <div class="modal-header mb-4">
            <div class="flex items-center space-x-2">
              <div class="w-8 h-8 bg-blue-600 rounded-lg flex items-center justify-center shadow-lg shadow-blue-500/30">
                <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"></path>
                </svg>
              </div>
              <div>
                <h3 class="text-base font-bold text-white">{{ $t('password.title') }}</h3>
                <p class="text-blue-300/80 text-xs mt-0.5">{{ $t('password.subtitle') }}</p>
              </div>
            </div>
          </div>

          <!-- Error Message -->
          <div v-if="errorMessage" class="mb-3 p-2 bg-red-500/20 border border-red-500/50 rounded-lg">
            <div class="flex items-center space-x-2 text-red-400">
              <svg class="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
              </svg>
              <span class="text-xs font-medium">{{ errorMessage }}</span>
            </div>
          </div>

          <!-- Unified Password Toggle (only show when multiple archives) -->
          <div v-if="archivePaths.length > 1" class="mb-3">
            <label class="flex items-center space-x-2 cursor-pointer group">
              <div class="relative">
                <input
                  type="checkbox"
                  v-model="useUnifiedPassword"
                  class="sr-only peer"
                />
                <div class="w-9 h-5 bg-gray-700 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-blue-500/50 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-blue-600"></div>
              </div>
              <div class="flex items-center space-x-1.5">
                <svg class="w-4 h-4 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"></path>
                </svg>
                <span class="text-xs font-medium text-gray-200 group-hover:text-white transition-colors">
                  {{ $t('password.useUnified') }}
                </span>
              </div>
            </label>
            <p class="text-xs text-gray-400 mt-1.5 ml-11">
              {{ $t('password.unifiedHint') }}
            </p>
          </div>

          <!-- Unified Password Input -->
          <div v-if="useUnifiedPassword" class="mb-3">
            <div class="bg-gradient-to-br from-blue-500/10 to-indigo-500/10 border border-blue-500/30 rounded-xl p-3">
              <div class="flex items-center space-x-1.5 mb-2">
                <svg class="w-4 h-4 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"></path>
                </svg>
                <span class="text-xs font-semibold text-blue-300">{{ $t('password.unifiedPasswordLabel') }}</span>
                <span class="text-xs text-gray-400">({{ archivePaths.length }} {{ $t('password.archives') }})</span>
              </div>
              <div class="relative">
                <input
                  :type="showUnifiedPassword ? 'text' : 'password'"
                  v-model="unifiedPassword"
                  :placeholder="$t('password.unifiedPlaceholder')"
                  class="w-full px-3 py-2 pr-10 bg-gray-900/70 border border-blue-500/30 rounded-lg text-white text-sm placeholder-gray-500 focus:border-blue-400 focus:ring-2 focus:ring-blue-500/30 transition-all"
                  @keyup.enter="handleConfirm"
                  autofocus
                />
                <button
                  type="button"
                  @click="showUnifiedPassword = !showUnifiedPassword"
                  class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-blue-400 transition-colors"
                >
                  <svg v-if="showUnifiedPassword" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"></path>
                  </svg>
                  <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
                  </svg>
                </button>
              </div>
            </div>
          </div>

          <!-- Individual Password Inputs -->
          <div v-else class="space-y-2 mb-3 max-h-80 overflow-y-auto custom-scrollbar">
            <div
              v-for="(archivePath, index) in archivePaths"
              :key="archivePath"
              class="bg-gradient-to-br from-gray-800/60 to-gray-900/60 border border-gray-700/50 hover:border-blue-500/30 rounded-xl p-3 transition-all duration-200"
            >
              <div class="flex items-center space-x-1.5 mb-2">
                <svg class="w-3.5 h-3.5 text-blue-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4"></path>
                </svg>
                <span class="text-xs font-medium text-gray-200 truncate" :title="getFileName(archivePath)">
                  {{ getFileName(archivePath) }}
                </span>
              </div>
              <div class="relative">
                <input
                  :type="showPasswords[index] ? 'text' : 'password'"
                  v-model="passwords[index]"
                  :placeholder="$t('password.placeholder')"
                  class="w-full px-3 py-2 pr-10 bg-gray-900/70 border border-gray-700/50 rounded-lg text-white text-sm placeholder-gray-500 focus:border-blue-500/50 focus:ring-2 focus:ring-blue-500/30 transition-all"
                  @keyup.enter="handleConfirm"
                  :autofocus="index === 0"
                />
                <button
                  type="button"
                  @click="togglePasswordVisibility(index)"
                  class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-blue-400 transition-colors"
                >
                  <svg v-if="showPasswords[index]" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"></path>
                  </svg>
                  <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
                  </svg>
                </button>
              </div>
            </div>
          </div>

          <!-- Actions -->
          <div class="flex justify-end gap-2 pt-2">
            <button
              @click="handleCancel"
              class="px-3 py-2 bg-gray-700/80 hover:bg-gray-600/80 rounded-lg transition-all duration-200 text-xs font-medium flex items-center space-x-1.5 border border-gray-600/50"
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
              <span>{{ $t('common.cancel') }}</span>
            </button>
            <button
              @click="handleConfirm"
              :disabled="!hasAllPasswords"
              :class="[
                'px-3 py-2 rounded-lg transition-all duration-200 text-xs font-medium flex items-center space-x-1.5',
                hasAllPasswords
                  ? 'bg-blue-600 hover:bg-blue-700 text-white'
                  : 'bg-gray-700/50 text-gray-500 cursor-not-allowed'
              ]"
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
              <span>{{ $t('common.confirm') }}</span>
            </button>
          </div>
        </div>
      </div>
    </transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'

const props = defineProps<{
  archivePaths: string[]
  errorMessage?: string
}>()

const emit = defineEmits<{
  (e: 'confirm', passwords: Record<string, string>): void
  (e: 'cancel'): void
}>()

// Unified password mode
const useUnifiedPassword = ref(false)
const unifiedPassword = ref('')
const showUnifiedPassword = ref(false)

// Individual passwords for each archive
const passwords = ref<string[]>(props.archivePaths.map(() => ''))
const showPasswords = ref<boolean[]>(props.archivePaths.map(() => false))

// Check if all passwords are filled
const hasAllPasswords = computed(() => {
  if (useUnifiedPassword.value) {
    return unifiedPassword.value.trim().length > 0
  }
  return passwords.value.every(pwd => pwd.trim().length > 0)
})

// Watch unified password mode changes
watch(useUnifiedPassword, (newValue) => {
  if (newValue) {
    // Switching to unified mode - use first non-empty password if available
    const firstPassword = passwords.value.find(pwd => pwd.trim().length > 0)
    if (firstPassword) {
      unifiedPassword.value = firstPassword
    }
  } else {
    // Switching to individual mode - fill all with unified password if it exists
    if (unifiedPassword.value.trim().length > 0) {
      passwords.value = props.archivePaths.map(() => unifiedPassword.value)
    }
  }
})

// Get filename from path
function getFileName(path: string): string {
  return path.split(/[/\\]/).pop() || path
}

// Toggle password visibility
function togglePasswordVisibility(index: number): void {
  showPasswords.value[index] = !showPasswords.value[index]
}

// Handle confirm
function handleConfirm(): void {
  if (!hasAllPasswords.value) return

  const result: Record<string, string> = {}

  if (useUnifiedPassword.value) {
    // Use unified password for all archives
    props.archivePaths.forEach((path) => {
      result[path] = unifiedPassword.value
    })
  } else {
    // Use individual passwords
    props.archivePaths.forEach((path, index) => {
      result[path] = passwords.value[index]
    })
  }

  emit('confirm', result)
}

// Handle cancel
function handleCancel(): void {
  emit('cancel')
}
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
    transform: scale(0.95) translateY(-10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.animate-scale-in {
  animation: scale-in 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
}

/* Modal overlay */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.85);
  backdrop-filter: blur(12px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

/* Modal content */
.modal-content {
  background: linear-gradient(135deg, rgba(17, 24, 39, 0.98), rgba(31, 41, 55, 0.98));
  border-radius: 1.25rem;
  padding: 2rem;
  max-width: 520px;
  width: 90%;
  border: 1px solid rgba(59, 130, 246, 0.2);
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.9), 0 0 0 1px rgba(59, 130, 246, 0.1);
}

/* Custom scrollbar */
.custom-scrollbar::-webkit-scrollbar {
  width: 8px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: rgba(31, 41, 55, 0.5);
  border-radius: 4px;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: linear-gradient(180deg, rgba(59, 130, 246, 0.6), rgba(99, 102, 241, 0.6));
  border-radius: 4px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: linear-gradient(180deg, rgba(59, 130, 246, 0.8), rgba(99, 102, 241, 0.8));
}
</style>
