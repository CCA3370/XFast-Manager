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
