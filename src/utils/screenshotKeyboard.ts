/** Keep text entry and media controls independent of preview arrow navigation. */
export function isPreviewKeyboardTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false
  return Boolean(
    target.closest(
      'button:not(.preview-nav-btn), input, textarea, select, video, [contenteditable="true"]',
    ),
  )
}
