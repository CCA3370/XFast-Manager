import { Store } from '@tauri-apps/plugin-store'
import type { DoctorHistoryFile, DoctorRun } from '@/types/doctor'
import { addDoctorHistoryRun, emptyDoctorHistory, getDoctorHistoryForPath } from '@/utils/doctor'

const STORE_FILE = 'health-history.json'
const STORE_KEY = 'history'

let historyStore: Store | null = null

async function getStore(): Promise<Store> {
  historyStore ??= await Store.load(STORE_FILE)
  return historyStore
}

export async function loadDoctorHistory(): Promise<DoctorHistoryFile> {
  const store = await getStore()
  const value = await store.get<DoctorHistoryFile>(STORE_KEY)
  if (
    !value ||
    value.schemaVersion !== 1 ||
    !value.installationByPath ||
    !value.runsByInstallation
  ) {
    return emptyDoctorHistory()
  }
  return value
}

export async function loadDoctorRunsForPath(xplanePath: string): Promise<DoctorRun[]> {
  return getDoctorHistoryForPath(await loadDoctorHistory(), xplanePath)
}

export async function saveDoctorRun(xplanePath: string, run: DoctorRun): Promise<DoctorRun[]> {
  const store = await getStore()
  const updated = addDoctorHistoryRun(await loadDoctorHistory(), xplanePath, run)
  await store.set(STORE_KEY, updated)
  return updated.runsByInstallation[run.installationId] ?? []
}
