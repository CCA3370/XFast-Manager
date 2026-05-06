$ErrorActionPreference = "Stop"

$Repo = "CCA3370/XFast-Manager"

$Issues = @(
  @{
    Number = 168
    Summary = "Removed the first-run Log Analysis tip from the top navigation."
    Verification = @("npm run lint", "npm run build")
    Manual = @("Open the app and confirm Log Analysis no longer shows the first-run tip bubble.")
  },
  @{
    Number = 169
    Summary = "Hardened aircraft variant toggles so the main list and ACF manager stay in sync."
    Verification = @("cargo check --manifest-path src-tauri/Cargo.toml", "npm run lint")
    Manual = @(
      "Use an aircraft folder with multiple top-level ACF files.",
      "Disable and re-enable one variant, then confirm the row, main toggle, and ACF dialog agree."
    )
  },
  @{
    Number = 170
    Summary = "Gateway requests now retry temporary upstream failures and show a clear retry-later message."
    Verification = @("cargo check --manifest-path src-tauri/Cargo.toml", "npm run lint")
    Manual = @(
      "Open Gateway search/details/install while the upstream service returns a temporary failure.",
      "Confirm raw Bad Gateway HTML is not shown and the workflow remains retryable."
    )
  },
  @{
    Number = 171
    Summary = "Split ZIP, 7z, and RAR volumes are grouped as one addon source, with clearer missing-volume messages."
    Verification = @("cargo check --manifest-path src-tauri/Cargo.toml", "cargo test --manifest-path src-tauri/Cargo.toml --no-run")
    Manual = @(
      "Analyze complete multi-volume ZIP, 7z, and RAR inputs.",
      "Remove one required volume and confirm the missing-volume message names the missing part."
    )
  },
  @{
    Number = 177
    Summary = "Airport Flatten search selection is more reliable and can refresh stale flatten indexes before retrying."
    Verification = @("cargo check --manifest-path src-tauri/Cargo.toml", "node scripts/validate-i18n.mjs", "npm run lint")
    Manual = @(
      "Search EGKK and confirm the selected result stays open with editable sources.",
      "When default and custom sources exist for the same ICAO, confirm no source is modified until a specific source button is clicked."
    )
  },
  @{
    Number = 182
    Summary = "Atomic install staging now falls back when the X-Plane root cannot create the staging folder."
    Verification = @("cargo check --manifest-path src-tauri/Cargo.toml", "cargo test --manifest-path src-tauri/Cargo.toml --no-run")
    Manual = @(
      "Run an atomic install where the X-Plane root staging folder cannot be created.",
      "Confirm the install uses a fallback staging location and still completes or reports the real install error."
    )
  },
  @{
    Number = 183
    Summary = "Per-variant ACF toggles now resolve the current ACF/XFMA file by variant stem, even if the UI has stale state."
    Verification = @("cargo check --manifest-path src-tauri/Cargo.toml", "cargo test --manifest-path src-tauri/Cargo.toml --no-run", "npm run lint")
    Manual = @(
      "Open the ACF manager and rapidly toggle the same variant off and on.",
      "Confirm no file-not-found error appears and skunkcrafts_updater.cfg keeps its non-disabled lines."
    )
  },
  @{
    Number = 185
    Summary = "Addon update drawers now soften temporary SkunkCrafts, X-Updater, and Zibo upstream failures."
    Verification = @("cargo check --manifest-path src-tauri/Cargo.toml", "npm run lint")
    Manual = @(
      "Open update details for SkunkCrafts, X-Updater, and Zibo-backed aircraft while the remote service is unavailable.",
      "Confirm local aircraft details remain visible and only the update area shows the retry-later message."
    )
  }
)

foreach ($Issue in $Issues) {
  $BodyLines = @(
    "Fixed locally.",
    "",
    $Issue.Summary,
    "",
    "Verification commands:",
    ($Issue.Verification | ForEach-Object { "- $_" }),
    "",
    "Manual checks:",
    ($Issue.Manual | ForEach-Object { "- $_" })
  )
  $Body = ($BodyLines | ForEach-Object { $_ }) -join "`n"
  $TempFile = New-TemporaryFile

  try {
    Set-Content -LiteralPath $TempFile -Value $Body -Encoding UTF8
    gh issue comment $Issue.Number --repo $Repo --body-file $TempFile
    gh issue close $Issue.Number --repo $Repo --reason completed
  }
  finally {
    Remove-Item -LiteralPath $TempFile -Force -ErrorAction SilentlyContinue
  }
}
