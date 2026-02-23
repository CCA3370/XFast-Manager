/**
 * Unified API service layer for Tauri command invocation
 * Provides consistent error handling and type-safe command calls
 */

import { invoke } from '@tauri-apps/api/core'
import { parseApiError, getErrorMessage, type ApiError } from '@/types'

/**
 * Error class for API errors with structured information
 */
export class CommandError extends Error {
  public readonly code?: string
  public readonly details?: string

  constructor(message: string, apiError?: ApiError) {
    super(message)
    this.name = 'CommandError'
    this.code = apiError?.code
    this.details = apiError?.details
  }
}

/**
 * Invoke a Tauri command with unified error handling
 *
 * @param command - The command name to invoke
 * @param args - Optional arguments to pass to the command
 * @returns Promise resolving to the command result
 * @throws CommandError if the command fails
 *
 * @example
 * ```typescript
 * // Simple command
 * const path = await invokeCommand<string>('get_xplane_path')
 *
 * // Command with arguments
 * const result = await invokeCommand<AnalysisResult>('analyze_files', { paths: filePaths })
 * ```
 */
export async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    const result = await invoke<T>(command, args)

    // Check if the result itself is a structured error (for commands that return Result types)
    if (result && typeof result === 'object') {
      const apiError = parseApiError(JSON.stringify(result))
      if (apiError) {
        throw new CommandError(apiError.message, apiError)
      }
    }

    return result
  } catch (error) {
    // If it's already a CommandError, re-throw it
    if (error instanceof CommandError) {
      throw error
    }

    // Try to parse as structured API error
    const apiError = parseApiError(error)
    if (apiError) {
      throw new CommandError(apiError.message, apiError)
    }

    // Fall back to generic error message
    throw new CommandError(getErrorMessage(error))
  }
}

/**
 * Invoke a Tauri command that returns void/unit
 * Convenience wrapper for commands that don't return a value
 *
 * @param command - The command name to invoke
 * @param args - Optional arguments to pass to the command
 * @throws CommandError if the command fails
 */
export async function invokeVoidCommand(
  command: string,
  args?: Record<string, unknown>,
): Promise<void> {
  await invokeCommand<void>(command, args)
}

/**
 * Invoke a Tauri command with optional error suppression
 * Returns null instead of throwing on error - useful for non-critical operations
 *
 * @param command - The command name to invoke
 * @param args - Optional arguments to pass to the command
 * @returns Promise resolving to the result or null on error
 */
export async function tryInvokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  try {
    return await invokeCommand<T>(command, args)
  } catch {
    return null
  }
}
