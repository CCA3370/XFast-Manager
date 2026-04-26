/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_XFAST_VERCEL_API_BASE_URL?: string
  readonly VITE_XFAST_ISSUE_DRAFT_API_URL?: string
  readonly VITE_XFAST_ISSUE_REDIRECT_API_URL?: string
  readonly VITE_XFAST_RELEASE_REDIRECT_API_URL?: string
}

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<object, object, unknown>
  export default component
}
