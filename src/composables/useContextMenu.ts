import { ref } from 'vue'

export interface ContextMenuItem {
  id: string
  label: string
  icon?: string
  iconFill?: string
  children?: ContextMenuItem[]
  disabled?: boolean
  hidden?: boolean
  danger?: boolean
  dividerAfter?: boolean
}

// Module-level singleton state (only one menu open at a time)
const visible = ref(false)
const x = ref(0)
const y = ref(0)
const items = ref<ContextMenuItem[]>([])
const submenuDirection = ref<'left' | 'right'>('right')
let actionCallback: ((id: string) => void) | null = null

export function useContextMenu() {
  function show(event: MouseEvent, menuItems: ContextMenuItem[], onAction: (id: string) => void) {
    event.preventDefault()
    event.stopPropagation()
    x.value = event.clientX
    y.value = event.clientY
    items.value = menuItems
      .filter((item) => !item.hidden)
      .map((item) => ({
        ...item,
        children: item.children?.filter((child) => !child.hidden),
      }))
    actionCallback = onAction
    visible.value = true
  }

  function hide() {
    visible.value = false
    actionCallback = null
  }

  function handleAction(id: string) {
    if (actionCallback) {
      actionCallback(id)
    }
    hide()
  }

  return {
    visible,
    x,
    y,
    items,
    submenuDirection,
    show,
    hide,
    handleAction,
  }
}
