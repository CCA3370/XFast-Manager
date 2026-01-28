import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { InstallProgress } from '@/types'

export const useProgressStore = defineStore('progress', () => {
  const progress = ref<InstallProgress | null>(null)
  const displayPercentage = ref(0) // Smooth interpolated display percentage
  let animationFrameId: number | null = null

  // Animation parameters for smooth interpolation
  const LERP_FACTOR = 0.08  // Interpolation factor (smaller = smoother)
  const MIN_SPEED = 0.1     // Minimum movement speed (prevents stalling)
  const EPSILON = 0.01      // Precision threshold

  function animate() {
    if (!progress.value) {
      animationFrameId = null
      return
    }

    const target = progress.value.percentage
    const current = displayPercentage.value
    const diff = target - current

    if (Math.abs(diff) < EPSILON) {
      // Close enough to target
      displayPercentage.value = target
    } else {
      // Interpolate movement, but ensure minimum speed
      const step = Math.max(Math.abs(diff) * LERP_FACTOR, MIN_SPEED) * Math.sign(diff)
      displayPercentage.value = current + step

      // Prevent overshooting target
      if (diff > 0) {
        displayPercentage.value = Math.min(displayPercentage.value, target)
      } else {
        displayPercentage.value = Math.max(displayPercentage.value, target)
      }
    }

    animationFrameId = requestAnimationFrame(animate)
  }

  function startAnimation() {
    if (animationFrameId === null) {
      animationFrameId = requestAnimationFrame(animate)
    }
  }

  function stopAnimation() {
    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId)
      animationFrameId = null
    }
  }

  const formatted = computed(() => {
    if (!progress.value) {
      return {
        percentage: '0.0',
        processedMB: '0.0',
        totalMB: '0.0',
        taskName: '',
        currentFile: '',
        taskProgress: '0/0',
      }
    }

    return {
      percentage: displayPercentage.value.toFixed(1),
      processedMB: (progress.value.processedBytes / 1048576).toFixed(1),
      totalMB: (progress.value.totalBytes / 1048576).toFixed(1),
      taskName: progress.value.currentTaskName,
      currentFile: progress.value.currentFile || '',
      taskProgress: `${progress.value.currentTaskIndex + 1}/${progress.value.totalTasks}`,
    }
  })

  function update(p: InstallProgress) {
    progress.value = p
    startAnimation()
  }

  function reset() {
    stopAnimation()
    progress.value = null
    displayPercentage.value = 0
  }

  // Set percentage directly (used to ensure 100% is shown before completion)
  function setPercentage(percentage: number) {
    if (progress.value) {
      progress.value = {
        ...progress.value,
        percentage
      }
    } else {
      // Create a minimal progress object if none exists
      progress.value = {
        percentage,
        totalBytes: 0,
        processedBytes: 0,
        currentTaskIndex: 0,
        totalTasks: 1,
        currentTaskName: '',
        currentFile: null,
        phase: 'finalizing'
      }
    }
    // Also update display percentage immediately for direct sets
    displayPercentage.value = percentage
  }

  return {
    progress,
    displayPercentage,
    formatted,
    update,
    reset,
    setPercentage,
  }
})
