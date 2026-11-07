// Run the production filesystem inspection/repair tests independently of Tauri's
// Windows test executable, which can fail before reaching Rust's test harness.
import fs from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const harness = path.join(root, 'target', 'health-core-tests')
await fs.mkdir(path.join(harness, 'src'), { recursive: true })
await fs.writeFile(path.join(harness, 'Cargo.toml'), `[package]
name = "xfast-health-core-tests"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
walkdir = "2"
`)
const modules = ['doctor_inspection', 'doctor_installation', 'doctor_navdata', 'doctor_repairs'].map((name) =>
  `#[path = ${JSON.stringify(path.join(root, 'src-tauri', 'src', 'analysis', `${name}.rs`))}]
mod ${name.replace('doctor_', '')};`,
)
await fs.writeFile(path.join(harness, 'src', 'lib.rs'), '#![allow(dead_code)]\n' + modules.join('\n'))
const result = spawnSync('cargo', ['test', '--offline', '--manifest-path', path.join(harness, 'Cargo.toml')], {
  cwd: root, stdio: 'inherit', windowsHide: true,
})
if (result.error) throw result.error
process.exitCode = result.status ?? 1
