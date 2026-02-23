#!/usr/bin/env node
import { spawnSync } from 'child_process'

function install(pkg) {
  console.log(`Installing ${pkg} ...`)
  const res = spawnSync('npm', ['install', pkg, '--no-save'], { stdio: 'inherit' })
  if (res.error || res.status !== 0) {
    console.warn(`Failed to install ${pkg} (exit ${res.status}) - continuing`)
  }
}

if (process.platform !== 'darwin') {
  // Only needed for macOS native rollup packages
  process.exit(0)
}

const arch = process.arch // 'x64' | 'arm64' | ...
if (arch === 'x64') install('@rollup/rollup-darwin-x64')
else if (arch === 'arm64') install('@rollup/rollup-darwin-arm64')
else console.log(`Unknown darwin arch: ${arch} - skipping rollup native install`)
