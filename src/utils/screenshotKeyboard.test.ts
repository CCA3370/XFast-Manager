// @vitest-environment happy-dom
import { describe, expect, it } from 'vitest'
import { isPreviewKeyboardTarget } from './screenshotKeyboard'

describe('screenshot preview keyboard navigation', () => {
  it('keeps navigation active when a preview arrow button or its icon has focus', () => {
    const button = document.createElement('button')
    button.className = 'preview-nav-btn preview-nav-next'
    button.innerHTML = '<svg><path /></svg>'
    expect(isPreviewKeyboardTarget(button)).toBe(false)
    expect(isPreviewKeyboardTarget(button.querySelector('path'))).toBe(false)
  })

  it('leaves input, editor, video and other button keys to their controls', () => {
    for (const tag of ['input', 'textarea', 'select', 'video', 'button']) {
      expect(isPreviewKeyboardTarget(document.createElement(tag))).toBe(true)
    }
    const editor = document.createElement('div')
    editor.contentEditable = 'true'
    editor.innerHTML = '<span>editing</span>'
    expect(isPreviewKeyboardTarget(editor.firstElementChild)).toBe(true)
  })
})
