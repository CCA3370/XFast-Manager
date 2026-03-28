import fs from 'node:fs'
import path from 'node:path'

function readJson(relativePath) {
  const fullPath = path.resolve(relativePath)
  return JSON.parse(fs.readFileSync(fullPath, 'utf8'))
}

function readCargoVersion(relativePath) {
  const fullPath = path.resolve(relativePath)
  const content = fs.readFileSync(fullPath, 'utf8')
  const match = content.match(/^version\s*=\s*"([^"]+)"/m)

  if (!match?.[1]) {
    throw new Error(`Unable to find version in ${relativePath}`)
  }

  return match[1]
}

const versions = {
  packageJson: readJson('package.json').version,
  tauriConfig: readJson('src-tauri/tauri.conf.json').version,
  cargoToml: readCargoVersion('src-tauri/Cargo.toml'),
}

if (new Set(Object.values(versions)).size !== 1) {
  console.error('Version mismatch detected across release sources:')
  console.error(`  package.json: ${versions.packageJson}`)
  console.error(`  src-tauri/tauri.conf.json: ${versions.tauriConfig}`)
  console.error(`  src-tauri/Cargo.toml: ${versions.cargoToml}`)
  process.exit(1)
}

console.log(`Version consistency check passed: ${versions.packageJson}`)

const tauriConfig = readJson('src-tauri/tauri.conf.json')
const wixVersion = tauriConfig.bundle?.windows?.wix?.version

function parseSemverCore(version) {
  const match = String(version).trim().match(/^(\d+)\.(\d+)\.(\d+)(?:[-+].+)?$/)

  if (!match) {
    throw new Error(`Unsupported app version format: ${version}`)
  }

  return match.slice(1, 4).map((value) => Number.parseInt(value, 10))
}

function hasSemverExtra(version) {
  return /[-+]/.test(String(version).trim())
}

function validateWixVersion(version, appVersion) {
  const raw = String(version).trim()
  const parts = raw.split('.')

  if (parts.length < 3 || parts.length > 4 || parts.some((part) => !/^\d+$/.test(part))) {
    throw new Error(
      `bundle.windows.wix.version must be numeric major.minor.patch[.build], got: ${version}`,
    )
  }

  const numeric = parts.map((part) => Number.parseInt(part, 10))
  const [major, minor, patch, build] = numeric

  if (major > 255 || minor > 255 || patch > 65535 || (build ?? 0) > 65535) {
    throw new Error(
      `bundle.windows.wix.version exceeds MSI limits (major/minor <= 255, patch/build <= 65535): ${version}`,
    )
  }

  const appCore = parseSemverCore(appVersion)
  const wixCore = numeric.slice(0, 3)

  if (appCore.join('.') !== wixCore.join('.')) {
    throw new Error(
      `bundle.windows.wix.version must share the same major.minor.patch as app version (${appCore.join('.')}), got: ${version}`,
    )
  }

  return { parts, numeric, appCore }
}

if (wixVersion != null) {
  const wixMeta = validateWixVersion(wixVersion, versions.tauriConfig)
  if (!hasSemverExtra(versions.tauriConfig)) {
    const stableVariants = new Set([wixMeta.appCore.join('.'), `${wixMeta.appCore.join('.')}.0`])
    if (!stableVariants.has(String(wixVersion).trim())) {
      throw new Error(
        `Stable app version ${versions.tauriConfig} should not keep a prerelease-style bundle.windows.wix.version override (${wixVersion}). Remove it or reset it to ${wixMeta.appCore.join('.')} / ${wixMeta.appCore.join('.')}.0.`,
      )
    }
  }
}

if (hasSemverExtra(versions.tauriConfig) && wixVersion == null) {
  console.error(
    `Pre-release/build app version ${versions.tauriConfig} requires src-tauri/tauri.conf.json bundle.windows.wix.version for MSI builds.`,
  )
  process.exit(1)
}

if (wixVersion != null) {
  console.log(`WiX/MSI version check passed: ${wixVersion}`)
}
