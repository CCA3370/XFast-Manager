import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '@/stores/app'

class Logger {
  /**
   * Check if a message should be logged based on current log level
   */
  private shouldLog(messageLevel: 'basic' | 'full' | 'debug'): boolean {
    const store = useAppStore()
    const currentLevel = store.logLevel

    // Map levels to numeric values for comparison
    const levels = { basic: 0, full: 1, debug: 2 }
    return levels[messageLevel] <= levels[currentLevel]
  }

  /**
   * Log an info message (fire-and-forget)
   */
  info(message: string, context?: string): void {
    // Info messages default to 'full' level
    if (!this.shouldLog('full')) return

    invoke('log_from_frontend', { level: 'info', message, context })
      .catch((e) => console.debug('Failed to log info:', e))
  }

  /**
   * Log an error message (always logged, even in basic mode) (fire-and-forget)
   */
  error(message: string, context?: string): void {
    invoke('log_from_frontend', { level: 'error', message, context })
      .catch((e) => console.debug('Failed to log error:', e))
    console.error(`[${context ?? 'error'}]`, message)
  }

  /**
   * Log a user operation (fire-and-forget)
   */
  operation(action: string, details?: string): void {
    // Operations default to 'full' level
    if (!this.shouldLog('full')) return

    const message = details ? `${action}: ${details}` : action
    this.info(message, 'user-action')
  }

  /**
   * Log a basic-level message (always logged except in off mode) (fire-and-forget)
   */
  basic(message: string, context?: string): void {
    if (!this.shouldLog('basic')) return

    invoke('log_from_frontend', { level: 'info', message, context })
      .catch((e) => console.debug('Failed to log basic:', e))
  }

  /**
   * Log a debug-level message (only in debug mode) (fire-and-forget)
   */
  debug(message: string, context?: string): void {
    if (!this.shouldLog('debug')) return

    invoke('log_from_frontend', { level: 'debug', message, context })
      .catch((e) => console.debug('Failed to log debug:', e))
  }

  /**
   * Get recent log lines
   */
  async getRecentLogs(lines = 50): Promise<string[]> {
    try {
      return await invoke<string[]>('get_recent_logs', { lines })
    } catch (e) {
      console.error('Failed to get recent logs:', e)
      return []
    }
  }

  /**
   * Get all logs
   */
  async getAllLogs(): Promise<string> {
    try {
      return await invoke<string>('get_all_logs')
    } catch (e) {
      console.error('Failed to get all logs:', e)
      return ''
    }
  }

  /**
   * Get the log file path
   */
  async getLogPath(): Promise<string> {
    try {
      return await invoke<string>('get_log_path')
    } catch (e) {
      console.error('Failed to get log path:', e)
      return ''
    }
  }

  /**
   * Open the log folder in system file manager
   */
  async openLogFolder(): Promise<void> {
    try {
      await invoke('open_log_folder')
    } catch (e) {
      console.error('Failed to open log folder:', e)
      throw e
    }
  }

  /**
   * Copy all logs to clipboard
   */
  async copyLogsToClipboard(): Promise<boolean> {
    try {
      const logs = await this.getAllLogs()
      if (logs) {
        await navigator.clipboard.writeText(logs)
        return true
      }
      return false
    } catch (e) {
      console.error('Failed to copy logs to clipboard:', e)
      return false
    }
  }
}

export const logger = new Logger()

// Convenience exports
export const logInfo = (message: string, context?: string) => logger.info(message, context)
export const logError = (message: string, context?: string) => logger.error(message, context)
export const logOperation = (action: string, details?: string) => logger.operation(action, details)
export const logBasic = (message: string, context?: string) => logger.basic(message, context)
export const logDebug = (message: string, context?: string) => logger.debug(message, context)
