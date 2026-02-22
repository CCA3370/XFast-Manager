/**
 * Utility for creating tracked timeouts that can be cleaned up on component unmount
 */

/**
 * Creates a tracked timeout that will be automatically cleaned up
 *
 * @param callback - Function to execute after the delay
 * @param delay - Delay in milliseconds
 * @param trackingSet - Set to track active timeout IDs for cleanup
 * @returns Timeout ID that can be used with clearTimeout
 *
 * @example
 * ```typescript
 * const activeTimeoutIds = new Set<ReturnType<typeof setTimeout>>()
 *
 * setTrackedTimeout(() => {
 *   console.log('Executed after delay')
 * }, 1000, activeTimeoutIds)
 *
 * // On component unmount:
 * onBeforeUnmount(() => {
 *   activeTimeoutIds.forEach((id) => clearTimeout(id))
 *   activeTimeoutIds.clear()
 * })
 * ```
 */
export function setTrackedTimeout(
  callback: () => void,
  delay: number,
  trackingSet: Set<ReturnType<typeof setTimeout>>,
): ReturnType<typeof setTimeout> {
  const id = setTimeout(() => {
    callback()
    // Remove the timeout ID from the tracking set after it fires
    trackingSet.delete(id)
  }, delay)
  trackingSet.add(id)
  return id
}
