<script setup lang="ts">
import { computed } from 'vue'
import AnimatedText from './AnimatedText.vue'

interface Props {
  /** Whether the section is expanded */
  expanded: boolean
  /** Title text (can be i18n key if using $t) */
  title: string
  /** Description text (can be i18n key if using $t) */
  description: string
  /** Icon color theme: green, blue, purple, amber, red, gray, indigo, rose, cyan */
  iconColor?: 'green' | 'blue' | 'purple' | 'amber' | 'red' | 'gray' | 'indigo' | 'rose' | 'cyan'
  /** Whether to round only top corners (for cards in a group) */
  roundedTop?: boolean
  /** Whether to round only bottom corners (for last card in a group) */
  roundedBottom?: boolean
  /** Optional badge text to show next to title */
  badge?: string
  /** Badge color variant */
  badgeColor?: 'green' | 'blue' | 'amber' | 'red' | 'gray'
}

const props = withDefaults(defineProps<Props>(), {
  iconColor: 'gray',
  roundedTop: false,
  roundedBottom: false,
  badge: undefined,
  badgeColor: 'gray',
})

const emit = defineEmits<{
  (e: 'toggle'): void
}>()

// Color classes for icon container
const iconColorClasses = computed(() => {
  const colors: Record<string, string> = {
    green: 'bg-green-100 dark:bg-green-500/10 text-green-600 dark:text-green-400',
    blue: 'bg-blue-100 dark:bg-blue-500/10 text-blue-600 dark:text-blue-400',
    purple: 'bg-purple-100 dark:bg-purple-500/10 text-purple-600 dark:text-purple-400',
    amber: 'bg-amber-100 dark:bg-amber-500/10 text-amber-600 dark:text-amber-400',
    red: 'bg-red-100 dark:bg-red-500/10 text-red-600 dark:text-red-400',
    gray: 'bg-gray-100 dark:bg-gray-500/10 text-gray-600 dark:text-gray-400',
    indigo: 'bg-indigo-100 dark:bg-indigo-500/10 text-indigo-600 dark:text-indigo-400',
    rose: 'bg-rose-100 dark:bg-rose-500/10 text-rose-600 dark:text-rose-400',
    cyan: 'bg-cyan-100 dark:bg-cyan-500/10 text-cyan-600 dark:text-cyan-400',
  }
  return colors[props.iconColor] || colors.gray
})

// Badge color classes
const badgeColorClasses = computed(() => {
  const colors: Record<string, string> = {
    green: 'bg-green-100 dark:bg-green-500/20 text-green-700 dark:text-green-300',
    blue: 'bg-blue-100 dark:bg-blue-500/20 text-blue-700 dark:text-blue-300',
    amber: 'bg-amber-100 dark:bg-amber-500/20 text-amber-700 dark:text-amber-300',
    red: 'bg-red-100 dark:bg-red-500/20 text-red-700 dark:text-red-300',
    gray: 'bg-gray-100 dark:bg-gray-500/20 text-gray-700 dark:text-gray-300',
  }
  return colors[props.badgeColor] || colors.gray
})

// Compute rounded corners class
const roundedClass = computed(() => {
  if (props.roundedTop && props.roundedBottom) return 'rounded-xl'
  if (props.roundedTop) return 'rounded-t-xl'
  if (props.roundedBottom) return 'rounded-b-xl'
  return 'rounded-xl'
})
</script>

<template>
  <div class="bg-white dark:bg-gray-800/50 shadow-sm" :class="roundedClass">
    <!-- Header (always visible, clickable) -->
    <div
      class="p-4 flex items-center justify-between cursor-pointer select-none hover:bg-gray-50 dark:hover:bg-gray-700/30 transition-colors"
      :class="[roundedTop ? 'rounded-t-xl' : roundedClass]"
      @click="emit('toggle')"
    >
      <div class="flex items-center space-x-3">
        <!-- Icon slot or default icon container -->
        <div
          class="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0"
          :class="iconColorClasses"
        >
          <slot name="icon">
            <!-- Default icon (cog/settings) -->
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
              />
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
              />
            </svg>
          </slot>
        </div>

        <!-- Title and description -->
        <div>
          <div class="flex items-center space-x-2">
            <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
              <AnimatedText>{{ title }}</AnimatedText>
            </h3>
            <span
              v-if="badge"
              class="px-2 py-0.5 text-xs font-medium rounded-full"
              :class="badgeColorClasses"
            >
              {{ badge }}
            </span>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400">
            <AnimatedText>{{ description }}</AnimatedText>
          </p>
        </div>
      </div>

      <!-- Expand/Collapse indicator -->
      <svg
        class="w-5 h-5 text-gray-400 dark:text-gray-500 transition-transform duration-200"
        :class="{ 'rotate-180': expanded }"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M19 9l-7 7-7-7"
        ></path>
      </svg>
    </div>

    <!-- Collapsible content -->
    <transition name="collapse">
      <div v-if="expanded" class="border-t border-gray-100 dark:border-gray-700/50">
        <slot></slot>
      </div>
    </transition>
  </div>
</template>

<style scoped>
/* Collapse animation */
.collapse-enter-active,
.collapse-leave-active {
  transition: all 0.2s ease-out;
  overflow: hidden;
}

.collapse-enter-from,
.collapse-leave-to {
  opacity: 0;
  max-height: 0;
}

.collapse-enter-to,
.collapse-leave-from {
  opacity: 1;
  max-height: 1000px;
}
</style>
