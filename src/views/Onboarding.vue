<template>
  <div class="h-full flex flex-col px-8 py-5 relative overflow-x-hidden select-none">
    <div class="flex-1 flex items-center justify-center overflow-x-hidden">
      <div class="w-full max-w-2xl flex flex-col min-h-[440px]">
        <div class="text-center mb-6">
          <h2 class="text-xl font-bold text-gray-900 dark:text-white">{{ $t('onboarding.title') }}</h2>
          <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">{{ $t('onboarding.subtitle') }}</p>
        </div>

        <div class="h-[300px] overflow-y-auto overflow-x-hidden onboarding-step-shell">
          <div class="overflow-x-hidden">
            <Transition :name="transitionName" mode="out-in">
              <div :key="currentStep.key" class="bg-white/80 dark:bg-gray-800/40 backdrop-blur-md border border-gray-200 dark:border-white/5 rounded-xl shadow-sm dark:shadow-md p-5">
              <div class="flex items-start justify-between gap-4">
                <div class="flex-1">
                  <h3 class="text-sm font-semibold text-gray-900 dark:text-white flex items-center gap-2">
                    <span>{{ $t(currentStep.titleKey) }}</span>
                  </h3>
                  <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                    {{ $t(currentStep.descKey) }}
                  </p>
                </div>
                <button
                  v-if="currentStep.key !== 'installPreferences' && currentStep.key !== 'xplanePath' && currentStep.key !== 'aircraftBackup'"
                  @click="toggleCurrent"
                  :disabled="isSubmitting || currentStep.disabled"
                  class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                  :class="currentStep.enabled ? currentStep.onClass : 'bg-gray-300 dark:bg-gray-600'"
                >
                  <span
                    class="inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform"
                    :class="currentStep.enabled ? 'translate-x-4.5' : 'translate-x-0.5'"
                  ></span>
                </button>
              </div>

              <ul v-if="currentStep.benefits?.length" class="text-xs text-gray-700 dark:text-gray-200 mt-4 space-y-2">
                <li v-for="benefit in currentStep.benefits" :key="benefit" class="flex items-start gap-2">
                  <svg class="w-4 h-4 text-emerald-500 dark:text-emerald-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  <span>{{ $t(benefit) }}</span>
                </li>
              </ul>

              <div v-if="currentStep.key === 'xplanePath'" class="mt-4 space-y-3">
                <div
                  class="flex items-center bg-gray-50 dark:bg-gray-900/50 border rounded-lg overflow-hidden focus-within:border-blue-500 dark:focus-within:border-blue-500 transition-colors duration-200"
                  :class="pathError ? 'border-red-500 dark:border-red-500' : 'border-gray-200 dark:border-gray-700/50'"
                >
                  <input
                    v-model="xplanePathInput"
                    type="text"
                    placeholder="C:\\X-Plane 12"
                    class="flex-1 px-4 py-2.5 bg-transparent border-none text-sm text-gray-900 dark:text-gray-200 placeholder-gray-400 dark:placeholder-gray-600 focus:outline-none focus:ring-0 select-text"
                    @blur="handlePathBlur"
                  />
                  <button
                    type="button"
                    @click.stop.prevent="selectFolder"
                    class="px-4 py-1.5 m-1 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 text-xs font-medium rounded-md transition-colors duration-200 flex items-center space-x-1.5 flex-shrink-0 border border-gray-300 dark:border-gray-600"
                  >
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path>
                    </svg>
                    <span>{{ $t('common.browse') }}</span>
                  </button>
                </div>
                <p v-if="pathError" class="text-xs text-red-500 dark:text-red-400">
                  {{ pathError }}
                </p>
                <p v-else class="text-xs text-gray-500 dark:text-gray-400">
                  {{ $t('settings.xplanePathDesc') }}
                </p>
              </div>

              <div v-if="currentStep.key === 'installPreferences'" class="mt-4 space-y-3">
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                  <div v-for="type in addonTypes" :key="type" class="flex items-center justify-between p-2 rounded-lg bg-gray-50 dark:bg-gray-900/30 border border-gray-100 dark:border-white/5">
                    <span class="text-xs font-medium text-gray-700 dark:text-gray-300 truncate mr-2" :title="getAddonTypeName(type)">
                      {{ getAddonTypeName(type) }}
                    </span>
                    <button
                      @click="toggleInstallPreference(type)"
                      class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0"
                      :class="installPreferences[type] ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600'"
                    >
                      <span
                        class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                        :class="installPreferences[type] ? 'translate-x-3.5' : 'translate-x-0.5'"
                      />
                    </button>
                  </div>
                </div>
              </div>

              <div v-if="currentStep.key === 'aircraftBackup'" class="mt-4 space-y-3">
                <label class="text-xs font-medium text-gray-700 dark:text-gray-300">
                  {{ $t('settings.configFilePatterns') }}
                </label>
                <div class="space-y-1.5">
                  <div v-for="(_pattern, index) in configPatterns" :key="index">
                    <div
                      class="flex items-center gap-2 p-2 bg-gray-50 dark:bg-gray-900/30 rounded-lg border transition-colors"
                      :class="patternErrors[index] ? 'border-red-300 dark:border-red-500/50' : 'border-gray-100 dark:border-white/5'"
                    >
                      <input
                        v-model="configPatterns[index]"
                        type="text"
                        class="flex-1 px-2 py-1 text-xs bg-white dark:bg-gray-800 border rounded text-gray-900 dark:text-gray-200 focus:outline-none focus:ring-1 focus:ring-blue-500 transition-colors select-text"
                        :class="patternErrors[index] ? 'border-red-300 dark:border-red-500' : 'border-gray-200 dark:border-gray-700'"
                        placeholder="*_prefs.txt"
                        @blur="handlePatternBlur"
                      >
                      <button
                        @click="removePattern(index)"
                        class="p-1 text-red-500 hover:bg-red-50 dark:hover:bg-red-500/10 rounded transition-colors"
                        :title="$t('common.delete')"
                      >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                      </button>
                    </div>
                    <p v-if="patternErrors[index]" class="mt-0.5 ml-2 text-xs text-red-500 dark:text-red-400">
                      {{ patternErrors[index] }}
                    </p>
                  </div>
                </div>
                <button
                  @click="addPattern"
                  class="px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors text-xs"
                >
                  {{ $t('settings.addPattern') }}
                </button>
              </div>

              <div v-if="currentStep.key === 'verificationPreferences'" class="mt-4 space-y-3">
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                  <div class="flex items-center justify-between p-2 rounded-lg bg-gray-50 dark:bg-gray-900/30 border border-gray-100 dark:border-white/5">
                    <span class="text-xs font-medium text-gray-700 dark:text-gray-300 truncate mr-2">{{ $t('settings.verifyZip') }}</span>
                    <button
                      @click="toggleVerificationPreference('zip')"
                      :disabled="!verificationEnabled"
                      class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0 disabled:opacity-40 disabled:cursor-not-allowed"
                      :class="verificationPreferences.zip ? 'bg-purple-500' : 'bg-gray-300 dark:bg-gray-600'"
                    >
                      <span
                        class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                        :class="verificationPreferences.zip ? 'translate-x-3.5' : 'translate-x-0.5'"
                      />
                    </button>
                  </div>
                  <div class="flex items-center justify-between p-2 rounded-lg bg-gray-50 dark:bg-gray-900/30 border border-gray-100 dark:border-white/5">
                    <span class="text-xs font-medium text-gray-700 dark:text-gray-300 truncate mr-2">{{ $t('settings.verify7z') }}</span>
                    <button
                      @click="toggleVerificationPreference('7z')"
                      :disabled="!verificationEnabled"
                      class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0 disabled:opacity-40 disabled:cursor-not-allowed"
                      :class="verificationPreferences['7z'] ? 'bg-purple-500' : 'bg-gray-300 dark:bg-gray-600'"
                    >
                      <span
                        class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                        :class="verificationPreferences['7z'] ? 'translate-x-3.5' : 'translate-x-0.5'"
                      />
                    </button>
                  </div>
                  <div class="flex items-center justify-between p-2 rounded-lg bg-gray-50 dark:bg-gray-900/30 border border-gray-100 dark:border-white/5">
                    <span class="text-xs font-medium text-gray-700 dark:text-gray-300 truncate mr-2">{{ $t('settings.verifyRar') }}</span>
                    <button
                      disabled
                      class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0 opacity-40 cursor-not-allowed bg-gray-300 dark:bg-gray-600"
                    >
                      <span
                        class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm translate-x-0.5"
                      />
                    </button>
                  </div>
                  <div class="flex items-center justify-between p-2 rounded-lg bg-gray-50 dark:bg-gray-900/30 border border-gray-100 dark:border-white/5">
                    <span class="text-xs font-medium text-gray-700 dark:text-gray-300 truncate mr-2">{{ $t('settings.verifyDirectory') }}</span>
                    <button
                      @click="toggleVerificationPreference('directory')"
                      :disabled="!verificationEnabled"
                      class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0 disabled:opacity-40 disabled:cursor-not-allowed"
                      :class="verificationPreferences.directory ? 'bg-purple-500' : 'bg-gray-300 dark:bg-gray-600'"
                    >
                      <span
                        class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                        :class="verificationPreferences.directory ? 'translate-x-3.5' : 'translate-x-0.5'"
                      />
                    </button>
                  </div>
                </div>
                <p class="text-xs text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-500/10 p-2 rounded-lg border border-amber-200 dark:border-amber-500/20">
                  {{ $t('settings.rarVerificationNote') }}
                </p>
              </div>

              <div v-if="currentStep.key === 'autoUpdateCheck'" class="mt-4">
                <div class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900/30 rounded-lg border border-gray-100 dark:border-white/5">
                  <div class="flex-1">
                    <label class="text-xs font-medium text-gray-700 dark:text-gray-300">
                      {{ $t('update.includePreRelease') }}
                    </label>
                    <p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                      {{ $t('update.includePreReleaseDesc') }}
                    </p>
                  </div>
                  <button
                    @click="toggleIncludePreRelease"
                    :disabled="!autoUpdateEnabled"
                    class="relative inline-flex h-4 w-7 items-center rounded-full transition-colors duration-200 focus:outline-none flex-shrink-0 ml-3 disabled:opacity-40 disabled:cursor-not-allowed"
                    :class="includePreReleaseEnabled ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600'"
                  >
                    <span
                      class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform duration-200 shadow-sm"
                      :class="includePreReleaseEnabled ? 'translate-x-3.5' : 'translate-x-0.5'"
                    />
                  </button>
                </div>
              </div>

              <p v-if="currentStep.noteKey && currentStep.key !== 'verificationPreferences'" class="text-xs text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-500/10 p-2 rounded-lg border border-amber-200 dark:border-amber-500/20 mt-4">
                {{ $t(currentStep.noteKey) }}
              </p>
              </div>
            </Transition>
          </div>
        </div>

        <div class="mt-4 flex items-center justify-between">
          <div class="text-xs text-gray-500 dark:text-gray-400">
            {{ currentIndex + 1 }} / {{ steps.length }}
          </div>
          <div class="flex items-center gap-2">
            <button
              v-if="currentIndex > 0"
              @click="prevStep"
              :disabled="isSubmitting"
              class="px-3 py-1.5 rounded-lg border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors text-sm select-none"
            >
              {{ $t('onboarding.back') }}
            </button>
            <button
              v-if="!isLastStep"
              @click="nextStep"
              :disabled="isSubmitting || !canProceed"
              class="px-3 py-1.5 rounded-lg bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm select-none"
            >
              {{ $t('onboarding.next') }}
            </button>
            <button
              v-else
              @click="finishOnboarding"
              :disabled="isSubmitting"
              class="px-3 py-1.5 rounded-lg bg-emerald-500 text-white hover:bg-emerald-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm select-none"
            >
              {{ $t('onboarding.finish') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useUpdateStore } from '@/stores/update'
import { useModalStore } from '@/stores/modal'
import { useToastStore } from '@/stores/toast'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { AddonType } from '@/types'

const { t } = useI18n()
const router = useRouter()
const store = useAppStore()
const updateStore = useUpdateStore()
const modal = useModalStore()
const toast = useToastStore()

const isSubmitting = ref(false)

const windowsIntegrationEnabled = ref(store.isContextMenuRegistered)
const atomicInstallEnabled = ref(store.atomicInstallEnabled)
const deleteSourceEnabled = ref(store.deleteSourceAfterInstall)
const autoUpdateEnabled = ref(updateStore.autoCheckEnabled)
const includePreReleaseEnabled = ref(updateStore.includePreRelease)
const autoSortSceneryEnabled = ref(store.autoSortScenery)
const xplanePathInput = ref(store.xplanePath)
const pathError = ref<string | null>(null)
const xplanePathValid = ref(!!store.xplanePath)
const installPreferences = ref({ ...store.installPreferences })
const verificationEnabled = ref(Object.values(store.verificationPreferences).some(Boolean))
const verificationPreferences = ref<Record<string, boolean>>({ ...store.verificationPreferences, rar: false })
const configPatterns = ref<string[]>([...store.getConfigFilePatterns()])
const patternErrors = ref<Record<number, string>>({})

const addonTypes = [AddonType.Aircraft, AddonType.Scenery, AddonType.SceneryLibrary, AddonType.Plugin, AddonType.Navdata, AddonType.Livery]
const verificationTypes = ['zip', '7z', 'rar', 'directory'] as const

const transitionDirection = ref<'forward' | 'backward'>('forward')
const transitionName = computed(() => (transitionDirection.value === 'forward' ? 'onboarding-slide-left' : 'onboarding-slide-right'))

const steps = computed(() => {
  const items: Array<{
    key: string
    titleKey: string
    descKey: string
    benefits?: string[]
    noteKey?: string
    onClass: string
    disabled: boolean
    isEnabled: () => boolean
    toggle: () => void
  }> = []
  items.push({
    key: 'xplanePath',
    titleKey: 'settings.xplanePath',
    descKey: 'settings.xplanePathDesc',
    onClass: 'bg-blue-600',
    disabled: false,
    isEnabled: () => xplanePathValid.value,
    toggle: () => {}
  })

  if (store.isWindows) {
    items.push({
      key: 'windowsIntegration',
      titleKey: 'settings.windowsIntegration',
      descKey: 'settings.windowsIntegrationDesc',
      benefits: [
        'settings.windowsIntegrationBenefit1',
        'settings.windowsIntegrationBenefit2',
        'settings.windowsIntegrationBenefit3',
        'settings.windowsIntegrationBenefit4'
      ],
      onClass: 'bg-blue-600',
      disabled: false,
      isEnabled: () => windowsIntegrationEnabled.value,
      toggle: () => { windowsIntegrationEnabled.value = !windowsIntegrationEnabled.value }
    })
    items.push({
      key: 'atomicInstall',
      titleKey: 'settings.atomicInstallTitle',
      descKey: 'settings.atomicInstallDesc',
      benefits: [
        'settings.atomicInstallBenefit1',
        'settings.atomicInstallBenefit2',
        'settings.atomicInstallBenefit3',
        'settings.atomicInstallBenefit4'
      ],
      noteKey: 'settings.atomicInstallNote',
      onClass: 'bg-indigo-500',
      disabled: false,
      isEnabled: () => atomicInstallEnabled.value,
      toggle: () => { atomicInstallEnabled.value = !atomicInstallEnabled.value }
    })
    items.push({
      key: 'deleteSource',
      titleKey: 'settings.deleteSourceTitle',
      descKey: 'settings.deleteSourceDesc',
      benefits: [
        'settings.deleteSourceBenefit1',
        'settings.deleteSourceBenefit2',
        'settings.deleteSourceBenefit3',
        'settings.deleteSourceBenefit4'
      ],
      onClass: 'bg-red-500',
      disabled: false,
      isEnabled: () => deleteSourceEnabled.value,
      toggle: () => { deleteSourceEnabled.value = !deleteSourceEnabled.value }
    })
  }

  items.push({
    key: 'installPreferences',
    titleKey: 'settings.installPreferences',
    descKey: 'settings.installPreferencesDesc',
    onClass: 'bg-green-500',
    disabled: false,
    isEnabled: () => true,
    toggle: () => {}
  })

  items.push({
    key: 'aircraftBackup',
    titleKey: 'settings.aircraftBackup',
    descKey: 'settings.aircraftBackupDesc',
    onClass: 'bg-blue-500',
    disabled: false,
    isEnabled: () => true,
    toggle: () => {}
  })

  items.push({
    key: 'verificationPreferences',
    titleKey: 'settings.verificationPreferences',
    descKey: 'settings.verificationPreferencesDesc',
    noteKey: 'settings.rarVerificationNote',
    onClass: 'bg-purple-500',
    disabled: false,
    isEnabled: () => verificationEnabled.value,
    toggle: () => toggleAllVerificationPreferences()
  })

  items.push({
    key: 'autoUpdateCheck',
    titleKey: 'update.autoUpdateCheck',
    descKey: 'update.autoUpdateCheckDesc',
    benefits: [
      'update.autoUpdateCheckBenefit1',
      'update.autoUpdateCheckBenefit2',
      'update.autoUpdateCheckBenefit3'
    ],
    onClass: 'bg-green-500',
    disabled: false,
    isEnabled: () => autoUpdateEnabled.value,
    toggle: () => { autoUpdateEnabled.value = !autoUpdateEnabled.value }
  })

  items.push({
    key: 'autoSortScenery',
    titleKey: 'settings.sceneryAutoSort',
    descKey: 'settings.sceneryAutoSortDesc',
    benefits: [
      'settings.sceneryAutoSortBenefit1',
      'settings.sceneryAutoSortBenefit2',
      'settings.sceneryAutoSortBenefit3'
    ],
    onClass: 'bg-cyan-500',
    disabled: false,
    isEnabled: () => autoSortSceneryEnabled.value,
    toggle: () => { autoSortSceneryEnabled.value = !autoSortSceneryEnabled.value }
  })

  return items
})

const currentIndex = ref(0)
const currentStep = computed(() => {
  const step = steps.value[currentIndex.value]
  return {
    ...step,
    enabled: step.isEnabled()
  }
})
const canProceed = computed(() => (currentStep.value.key === 'xplanePath' ? xplanePathValid.value : true))
const isLastStep = computed(() => currentIndex.value === steps.value.length - 1)

function toggleCurrent() {
  if (currentStep.value.disabled) return
  currentStep.value.toggle()
}

function validateGlobPattern(pattern: string): string | null {
  if (!pattern || pattern.trim() === '') {
    return null
  }

  let bracketDepth = 0
  let braceDepth = 0

  for (let i = 0; i < pattern.length; i++) {
    const char = pattern[i]
    const prevChar = i > 0 ? pattern[i - 1] : ''

    if (prevChar === '\\') continue

    if (char === '[') bracketDepth++
    if (char === ']') bracketDepth--
    if (char === '{') braceDepth++
    if (char === '}') braceDepth--

    if (bracketDepth < 0) return t('settings.patternUnbalancedBracket')
    if (braceDepth < 0) return t('settings.patternUnbalancedBrace')
  }

  if (bracketDepth !== 0) return t('settings.patternUnbalancedBracket')
  if (braceDepth !== 0) return t('settings.patternUnbalancedBrace')

  if (pattern.includes('//')) return t('settings.patternInvalidSlash')

  return null
}

function handlePatternBlur() {
  const errors: Record<number, string> = {}
  const validPatterns: string[] = []

  configPatterns.value.forEach((pattern, index) => {
    const trimmed = pattern.trim()
    if (trimmed === '') return

    const error = validateGlobPattern(trimmed)
    if (error) {
      errors[index] = error
    } else {
      validPatterns.push(trimmed)
    }
  })

  patternErrors.value = errors
  store.setConfigFilePatterns(validPatterns)
}

function addPattern() {
  configPatterns.value.push('')
}

function removePattern(index: number) {
  configPatterns.value.splice(index, 1)
  handlePatternBlur()
}

async function validateXplanePath(path: string): Promise<boolean> {
  if (path.trim() === '') {
    pathError.value = t('settings.pathError')
    xplanePathValid.value = false
    return false
  }

  try {
    const isValid = await invoke<boolean>('validate_xplane_path', { path })
    if (!isValid) {
      const exists = await invoke<boolean>('check_path_exists', { path })
      pathError.value = exists ? t('settings.notValidXplanePath') : t('settings.pathNotExist')
      xplanePathValid.value = false
      return false
    }
  } catch (error) {
    pathError.value = t('common.error')
    xplanePathValid.value = false
    return false
  }

  pathError.value = null
  xplanePathValid.value = true
  store.setXplanePath(path)
  return true
}

async function handlePathBlur() {
  await validateXplanePath(xplanePathInput.value)
}

async function selectFolder() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('settings.selectXplaneFolder')
    })

    if (selected) {
      const selectedPath = selected as string
      xplanePathInput.value = selectedPath
      await validateXplanePath(selectedPath)
    }
  } catch (error) {
    modal.showError(t('common.error') + ': ' + String(error))
  }
}

function getAddonTypeName(type: AddonType): string {
  switch (type) {
    case AddonType.Aircraft: return t('settings.typeAircraft')
    case AddonType.Scenery: return t('settings.typeScenery')
    case AddonType.SceneryLibrary: return t('settings.typeSceneryLibrary')
    case AddonType.Plugin: return t('settings.typePlugin')
    case AddonType.Navdata: return t('settings.typeNavdata')
    case AddonType.Livery: return t('settings.typeLivery')
    default: return type
  }
}

function toggleInstallPreference(type: AddonType) {
  installPreferences.value[type] = !installPreferences.value[type]
}

function toggleVerificationPreference(type: string) {
  if (!verificationEnabled.value || type === 'rar') return
  verificationPreferences.value[type] = !verificationPreferences.value[type]
}

function toggleAllVerificationPreferences() {
  const newValue = !verificationEnabled.value
  verificationEnabled.value = newValue
}

function toggleIncludePreRelease() {
  if (!autoUpdateEnabled.value) return
  includePreReleaseEnabled.value = !includePreReleaseEnabled.value
}

async function nextStep() {
  if (currentStep.value.key === 'xplanePath' && !xplanePathValid.value) {
    const ok = await validateXplanePath(xplanePathInput.value)
    if (!ok) return
  }
  if (currentIndex.value < steps.value.length - 1) {
    transitionDirection.value = 'forward'
    currentIndex.value += 1
  }
}

function prevStep() {
  if (currentIndex.value > 0) {
    transitionDirection.value = 'backward'
    currentIndex.value -= 1
  }
}

function applyInstallPreferences() {
  addonTypes.forEach(type => {
    if (store.installPreferences[type] !== installPreferences.value[type]) {
      store.togglePreference(type)
    }
  })
}

function applyVerificationPreferences() {
  verificationTypes.forEach(type => {
    const desired = verificationEnabled.value ? verificationPreferences.value[type] : false
    if (store.verificationPreferences[type] !== desired) {
      store.toggleVerificationPreference(type)
    }
  })
}

async function applyWindowsIntegration(enabled: boolean) {
  if (store.isContextMenuRegistered === enabled) return
  try {
    if (enabled) {
      await invoke('register_context_menu')
      store.isContextMenuRegistered = true
      toast.success(t('settings.contextMenuRegistered'))
    } else {
      await invoke('unregister_context_menu')
      store.isContextMenuRegistered = false
      toast.success(t('settings.contextMenuUnregistered'))
    }
  } catch (error) {
    const errorMsg = String(error).toLowerCase()
    if (enabled && (errorMsg.includes('already') || errorMsg.includes('exist'))) {
      store.isContextMenuRegistered = true
      toast.info(t('settings.contextMenuRegistered'))
      return
    }
    if (!enabled && (errorMsg.includes('not found') || errorMsg.includes('not exist') || errorMsg.includes('not registered'))) {
      store.isContextMenuRegistered = false
      toast.info(t('settings.contextMenuUnregistered'))
      return
    }
    throw error
  }
}

async function finishOnboarding() {
  if (isSubmitting.value) return
  isSubmitting.value = true
  try {
    if (store.isWindows) {
      await applyWindowsIntegration(windowsIntegrationEnabled.value)
      if (store.atomicInstallEnabled !== atomicInstallEnabled.value) {
        store.toggleAtomicInstall()
      }
      if (store.deleteSourceAfterInstall !== deleteSourceEnabled.value) {
        store.toggleDeleteSourceAfterInstall()
      }
    }

    applyInstallPreferences()
    applyVerificationPreferences()

    if (updateStore.autoCheckEnabled !== autoUpdateEnabled.value) {
      updateStore.toggleAutoCheck()
    }
    if (updateStore.includePreRelease !== includePreReleaseEnabled.value) {
      updateStore.toggleIncludePreRelease()
    }

    if (store.autoSortScenery !== autoSortSceneryEnabled.value) {
      store.toggleAutoSortScenery()
    }

    localStorage.setItem('onboardingCompleted', 'true')
    await router.replace('/')
  } catch (error) {
    modal.showError(t('common.error') + ': ' + String(error))
  } finally {
    isSubmitting.value = false
  }
}
</script>

<style scoped>
.onboarding-step-shell {
  overflow-x: hidden;
}
.onboarding-slide-left-enter-active,
.onboarding-slide-left-leave-active,
.onboarding-slide-right-enter-active,
.onboarding-slide-right-leave-active {
  transition: all 0.28s ease;
}

.onboarding-slide-left-enter-from {
  opacity: 0;
  transform: translateX(24px);
}

.onboarding-slide-left-leave-to {
  opacity: 0;
  transform: translateX(-24px);
}

.onboarding-slide-right-enter-from {
  opacity: 0;
  transform: translateX(-24px);
}

.onboarding-slide-right-leave-to {
  opacity: 0;
  transform: translateX(24px);
}

/* Prevent button text selection */
button {
  -webkit-user-select: none;
  -moz-user-select: none;
  -ms-user-select: none;
  user-select: none;
}
</style>
