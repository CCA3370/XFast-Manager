// Execute the production flight-plan filesystem tests without Tauri's Windows
// test executable, which can fail before reaching the Rust test harness.
import fs from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const harness = path.join(root, 'target', 'flight-plan-core-tests')
await fs.mkdir(path.join(harness, 'src'), { recursive: true })
await fs.writeFile(path.join(harness, 'Cargo.toml'), `[package]
name = "xfast-flight-plan-core-tests"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
`)
await fs.writeFile(path.join(harness, 'src', 'lib.rs'), `#![allow(dead_code)]
#[path = ${JSON.stringify(path.join(root, 'src-tauri', 'src', 'management', 'flight_plans.rs'))}]
mod flight_plans;
`)
const result = spawnSync('cargo', ['test', '--offline', '--manifest-path', path.join(harness, 'Cargo.toml')], {
  cwd: root, stdio: 'inherit', windowsHide: true,
})
if (result.error) throw result.error
process.exitCode = result.status ?? 1
