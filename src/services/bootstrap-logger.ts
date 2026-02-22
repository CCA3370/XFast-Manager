/**
 * Bootstrap Logger - Early initialization logging service
 *
 * This logger can be used BEFORE any Vue app, Pinia stores, or other services are initialized.
 * It directly calls Tauri backend without any store dependencies.
 *
 * Use this for:
 * - Logging during app bootstrap/initialization
 * - Capturing errors before the main app loads
 * - Global error handlers (window.onerror, unhandledrejection)
 */

import { invoke } from '@tauri-apps/api/core'

// Buffer for logs before Tauri is ready
const logBuffer: Array<{ level: string; message: string; context?: string }> = []
let isTauriReady = false
let flushAttempts = 0
const MAX_FLUSH_ATTEMPTS = 5

/**
 * Check if Tauri IPC is available
 */
function checkTauriReady(): boolean {
  try {
    // Check if window.__TAURI_INTERNALS__ exists (Tauri v2)
    return (
      typeof window !== 'undefined' &&
      (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !== undefined
    )
  } catch {
    return false
  }
}

/**
 * Flush buffered logs to Tauri backend
 */
async function flushBuffer(): Promise<void> {
  if (logBuffer.length === 0) return

  const logsToFlush = [...logBuffer]
  logBuffer.length = 0

  for (const log of logsToFlush) {
    try {
      await invoke('log_from_frontend', {
        level: log.level,
        message: log.message,
        context: log.context,
      })
    } catch (e) {
      // If invoke fails, put back in buffer and stop
      logBuffer.unshift(log)
      console.warn('[bootstrap-logger] Failed to flush log:', e)
      break
    }
  }
}

/**
 * Try to flush buffer with retry logic
 */
function tryFlushBuffer(): void {
  if (isTauriReady) {
    flushBuffer().catch(console.error)
    return
  }

  if (checkTauriReady()) {
    isTauriReady = true
    flushBuffer().catch(console.error)
    return
  }

  // Retry with exponential backoff
  if (flushAttempts < MAX_FLUSH_ATTEMPTS) {
    flushAttempts++
    setTimeout(tryFlushBuffer, 100 * flushAttempts)
  }
}

/**
 * Core logging function - works before and after Tauri is ready
 */
function log(level: string, message: string, context?: string): void {
  // Always log to console for immediate visibility
  const timestamp = new Date().toISOString()
  const prefix = `[${timestamp}] [${level.toUpperCase()}]${context ? ` [${context}]` : ''}`

  if (level === 'error') {
    console.error(prefix, message)
  } else if (level === 'debug') {
    console.debug(prefix, message)
  } else {
    console.log(prefix, message)
  }

  // Try to send to Tauri backend
  if (isTauriReady || checkTauriReady()) {
    isTauriReady = true
    invoke('log_from_frontend', { level, message, context }).catch((e) =>
      console.debug('[bootstrap-logger] Invoke failed:', e),
    )
  } else {
    // Buffer for later
    logBuffer.push({ level, message, context })
    tryFlushBuffer()
  }
}

// ============================================================================
// Public API
// ============================================================================

export function bootstrapInfo(message: string, context?: string): void {
  log('info', message, context ?? 'bootstrap')
}

export function bootstrapError(message: string, context?: string): void {
  log('error', message, context ?? 'bootstrap')
}

export function bootstrapDebug(message: string, context?: string): void {
  log('debug', message, context ?? 'bootstrap')
}

/**
 * Mark Tauri as ready and flush any buffered logs
 * Call this after Tauri is confirmed to be initialized
 */
export function markTauriReady(): void {
  isTauriReady = true
  flushBuffer().catch(console.error)
}

/**
 * Format an error object into a loggable string
 */
export function formatError(error: unknown): string {
  if (error instanceof Error) {
    return `${error.name}: ${error.message}${error.stack ? `\n${error.stack}` : ''}`
  }
  if (typeof error === 'string') {
    return error
  }
  try {
    return JSON.stringify(error)
  } catch {
    return String(error)
  }
}

/**
 * Setup global error handlers - call this as early as possible
 */
export function setupGlobalErrorHandlers(): void {
  // Handle uncaught errors
  window.onerror = (message, source, lineno, colno, error) => {
    const errorMsg = error ? formatError(error) : `${message} at ${source}:${lineno}:${colno}`
    bootstrapError(`Uncaught error: ${errorMsg}`, 'global-error')
    return false // Don't prevent default handling
  }

  // Handle unhandled promise rejections
  window.onunhandledrejection = (event) => {
    const reason = formatError(event.reason)
    bootstrapError(`Unhandled promise rejection: ${reason}`, 'unhandled-rejection')
  }

  bootstrapDebug('Global error handlers installed', 'bootstrap')
}
