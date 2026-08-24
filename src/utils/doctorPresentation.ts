export function formatDoctorDuration(milliseconds: number): string {
  const safeMilliseconds = Math.max(0, Math.round(milliseconds))
  if (safeMilliseconds < 1_000) return `${safeMilliseconds} ms`

  const totalSeconds = Math.round(safeMilliseconds / 1_000)
  if (totalSeconds < 60) return `${(safeMilliseconds / 1_000).toFixed(1)} s`

  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  return `${minutes}m ${seconds}s`
}
