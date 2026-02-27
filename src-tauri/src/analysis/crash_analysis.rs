use std::collections::HashMap;
use std::path::{Path, PathBuf};

use minidump::{Minidump, Module};
use minidump_processor::ProcessorOptions;
use minidump_unwind::MultiSymbolProvider;

use crate::log_debug;
use crate::logger;
use crate::LogIssue;

const LOG_CTX: &str = "crash_analysis";

// ========== Data Structures ==========

#[derive(serde::Serialize, Clone)]
pub struct CrashReportInfo {
    pub dmp_path: String,
    pub file_name: String,
    pub timestamp: u64,
    pub file_size: u64,
}

#[derive(serde::Serialize, Clone)]
pub struct CrashExceptionInfo {
    pub exception_type: String,
    pub exception_code: String,
    pub crash_address: String,
    pub crash_module: Option<String>,
    pub crash_module_offset: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct CrashStackFrame {
    pub frame_index: usize,
    pub module_name: Option<String>,
    pub offset: String,
    pub trust: String,
}

#[derive(serde::Serialize, Clone)]
pub struct CrashModuleInfo {
    pub name: String,
    pub filename: String,
    pub version: Option<String>,
    pub is_plugin: bool,
    pub base_address: String,
    pub size: u64,
}

#[derive(serde::Serialize, Clone)]
pub struct CrashCause {
    pub cause_key: String,
    pub score: f64,
    pub evidence: Vec<String>,
    pub blamed_module: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct DeepCrashAnalysis {
    pub report_info: CrashReportInfo,
    pub exception: Option<CrashExceptionInfo>,
    pub crash_stack: Vec<CrashStackFrame>,
    pub loaded_modules: Vec<CrashModuleInfo>,
    pub loaded_plugins: Vec<String>,
    pub crash_causes: Vec<CrashCause>,
    pub parse_success: bool,
    pub parse_warnings: Vec<String>,
}

// ========== Module Classification ==========

fn is_plugin(name: &str) -> bool {
    name.to_lowercase().ends_with(".xpl")
}

fn is_xplane_core(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("x-plane") || lower.contains("xplane")
}

fn is_gpu_driver(name: &str) -> bool {
    const GPU_DLLS: &[&str] = &[
        "nvoglv64",
        "nvoglv32",
        "nvcuda",
        "nvwgf2umx",
        "nvd3dumx",
        "nvapi64",
        "nvlddmkm",
        "atioglxx",
        "aticfx64",
        "aticfx32",
        "amdxc64",
        "atidxx64",
        "atidxx32",
        "amdihk64",
        "igdumdim64",
        "igdumdim32",
        "igd10iumd64",
        "igd12umd64",
        "igc64",
        "ig75icd64",
        "vulkan-1",
        "vulkan_radeon",
        "amdvlk64",
        "nvoglv",
    ];
    let lower = name.to_lowercase();
    let stem = lower.trim_end_matches(".dll");
    GPU_DLLS.iter().any(|d| stem == *d || lower.contains(d))
}

fn is_injected_dll(name: &str) -> bool {
    const INJECTED: &[&str] = &[
        "reshade",
        "dxgi.dll",
        "d3d11.dll",
        "gamepp",
        "rtsshoooks64",
        "rtsshooks64",
        "rtsshooks",
        "rtss",
        "nvidia share",
        "geforce",
        "msi afterburner",
        "fraps",
        "bandicam",
        "obs-vulkan",
        "obs-opengl",
        "steam_api",
        "gameoverlayrenderer",
    ];
    let lower = name.to_lowercase();
    INJECTED.iter().any(|d| lower.contains(d))
}

fn module_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

// ========== Core Functions ==========

pub fn find_most_recent_crash_report(
    xplane_path: &str,
    skip_date_check: bool,
) -> Option<PathBuf> {
    let reports_dir = Path::new(xplane_path)
        .join("Output")
        .join("crash_reports")
        .join("reports");

    if !reports_dir.is_dir() {
        log_debug!("Crash reports directory not found", &reports_dir.to_string_lossy());
        return None;
    }

    log_debug!("Scanning crash reports directory", &reports_dir.to_string_lossy());

    // Get Log.txt modification date for same-day filtering
    let log_date = if skip_date_check {
        logger::log_info(
            "Date check disabled (skip_date_check=true), will accept any .dmp file",
            Some(LOG_CTX),
        );
        None
    } else {
        let log_path = Path::new(xplane_path).join("Log.txt");
        match std::fs::metadata(&log_path)
            .ok()
            .and_then(|m| m.modified().ok())
        {
            Some(log_mod) => {
                let log_date = date_from_system_time(log_mod);
                logger::log_info(
                    &format!("Log.txt modification date: {}", log_date),
                    Some(LOG_CTX),
                );
                Some(log_date)
            }
            None => {
                logger::log_info(
                    "Cannot read Log.txt modification time, skipping date filter",
                    Some(LOG_CTX),
                );
                None
            }
        }
    };

    let mut dmp_count = 0u32;
    let mut skipped_date = 0u32;
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;

    if let Ok(entries) = std::fs::read_dir(&reports_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("dmp") {
                dmp_count += 1;
                if let Ok(meta) = path.metadata() {
                    if let Ok(modified) = meta.modified() {
                        // Check same-day constraint
                        if let Some(ref required_date) = log_date {
                            let dmp_date = date_from_system_time(modified);
                            if dmp_date != *required_date {
                                skipped_date += 1;
                                log_debug!(
                                    &format!(
                                        "Skipping {} (date {} != log date {})",
                                        path.file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_default(),
                                        dmp_date,
                                        required_date
                                    ),
                                    LOG_CTX
                                );
                                continue;
                            }
                        }
                        if newest.as_ref().map_or(true, |(_, t)| modified > *t) {
                            newest = Some((path, modified));
                        }
                    }
                }
            }
        }
    }

    logger::log_info(
        &format!(
            "Found {} crash dump file(s), {} skipped (different date), {} eligible",
            dmp_count,
            skipped_date,
            dmp_count - skipped_date
        ),
        Some(LOG_CTX),
    );

    if let Some((ref p, _)) = newest {
        logger::log_info(
            &format!("Most recent eligible crash dump: {}", p.to_string_lossy()),
            Some(LOG_CTX),
        );
    }

    newest.map(|(p, _)| p)
}

/// Extract YYYY-MM-DD date string from a SystemTime
fn date_from_system_time(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Calculate date from unix timestamp
    let days = (secs / 86400) as i64;
    // Algorithm from https://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

pub async fn parse_minidump(dmp_path: &Path) -> Result<ParsedMinidump, String> {
    logger::log_info(
        &format!("Reading minidump file: {}", dmp_path.to_string_lossy()),
        Some(LOG_CTX),
    );

    let dump = Minidump::read_path(dmp_path).map_err(|e| {
        let msg = format!("Failed to read minidump: {}", e);
        logger::log_error(&msg, Some(LOG_CTX));
        msg
    })?;

    logger::log_info("Minidump file loaded, starting processing", Some(LOG_CTX));

    let provider = MultiSymbolProvider::new();
    let opts = ProcessorOptions::default();

    let state = minidump_processor::process_minidump_with_options(&dump, &provider, opts)
        .await
        .map_err(|e| {
            let msg = format!("Failed to process minidump: {}", e);
            logger::log_error(&msg, Some(LOG_CTX));
            msg
        })?;

    logger::log_info(
        &format!(
            "Minidump processed: {} thread(s), {} module(s)",
            state.threads.len(),
            state.modules.iter().count()
        ),
        Some(LOG_CTX),
    );

    let mut warnings: Vec<String> = Vec::new();

    // Extract exception info
    let exception = if let Some(ref exc) = state.exception_info {
        let crash_mod = state
            .requesting_thread
            .and_then(|tid| state.threads.get(tid))
            .and_then(|t| t.frames.first())
            .and_then(|f| f.module.as_ref())
            .map(|m| module_name_from_path(&m.code_file()));

        let crash_offset = state
            .requesting_thread
            .and_then(|tid| state.threads.get(tid))
            .and_then(|t| t.frames.first())
            .map(|f| format!("0x{:x}", f.instruction));

        logger::log_info(
            &format!(
                "Exception: {} at address 0x{:x}, crash module: {}",
                exc.reason,
                *exc.address,
                crash_mod.as_deref().unwrap_or("unknown")
            ),
            Some(LOG_CTX),
        );

        Some(CrashExceptionInfo {
            exception_type: exc.reason.to_string(),
            exception_code: format!("0x{:x}", *exc.address),
            crash_address: format!("0x{:x}", *exc.address),
            crash_module: crash_mod,
            crash_module_offset: crash_offset,
        })
    } else {
        let warn = "No exception info found in crash dump".to_string();
        logger::log_info(&warn, Some(LOG_CTX));
        warnings.push(warn);
        None
    };

    // Extract crash stack from crashing thread
    let crash_stack: Vec<CrashStackFrame> = state
        .requesting_thread
        .and_then(|tid| state.threads.get(tid))
        .map(|thread| {
            logger::log_info(
                &format!(
                    "Crash thread has {} frame(s), extracting top 15",
                    thread.frames.len()
                ),
                Some(LOG_CTX),
            );
            thread
                .frames
                .iter()
                .take(15)
                .enumerate()
                .map(|(i, frame)| {
                    let module_name =
                        frame.module.as_ref().map(|m| module_name_from_path(&m.code_file()));
                    CrashStackFrame {
                        frame_index: i,
                        module_name,
                        offset: format!("0x{:x}", frame.instruction),
                        trust: format!("{:?}", frame.trust),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // Log stack frames
    for frame in &crash_stack {
        log_debug!(
            &format!(
                "  Frame #{}: {} @ {}",
                frame.frame_index,
                frame.module_name.as_deref().unwrap_or("???"),
                frame.offset
            ),
            LOG_CTX
        );
    }

    // Extract loaded modules
    let mut loaded_modules: Vec<CrashModuleInfo> = Vec::new();
    let mut loaded_plugins: Vec<String> = Vec::new();

    for module in state.modules.iter() {
        let code_file = module.code_file();
        let filename = module_name_from_path(&code_file);
        let is_plug = is_plugin(&filename);

        if is_plug {
            loaded_plugins.push(filename.clone());
        }

        let version = module.version().map(|v| v.to_string());

        loaded_modules.push(CrashModuleInfo {
            name: code_file.to_string(),
            filename,
            version,
            is_plugin: is_plug,
            base_address: format!("0x{:x}", module.base_address()),
            size: module.size(),
        });
    }

    logger::log_info(
        &format!(
            "Loaded {} module(s), {} plugin(s): [{}]",
            loaded_modules.len(),
            loaded_plugins.len(),
            loaded_plugins.join(", ")
        ),
        Some(LOG_CTX),
    );

    Ok(ParsedMinidump {
        exception,
        crash_stack,
        loaded_modules,
        loaded_plugins,
        warnings,
    })
}

pub struct ParsedMinidump {
    pub exception: Option<CrashExceptionInfo>,
    pub crash_stack: Vec<CrashStackFrame>,
    pub loaded_modules: Vec<CrashModuleInfo>,
    pub loaded_plugins: Vec<String>,
    pub warnings: Vec<String>,
}

// ========== Scoring Algorithm ==========

pub fn score_crash_causes(parsed: &ParsedMinidump, log_issues: &[LogIssue]) -> Vec<CrashCause> {
    logger::log_info("Starting crash cause scoring", Some(LOG_CTX));

    // cause_key -> (raw_score, evidence_list, blamed_module)
    let mut scores: HashMap<String, (f64, Vec<String>, Option<String>)> = HashMap::new();

    let mut add_evidence = |key: &str, points: f64, evidence: &str, blamed: Option<&str>| {
        log_debug!(
            &format!(
                "  Evidence: {} -> {} (+{} pts){}",
                evidence,
                key,
                points,
                blamed.map(|b| format!(", blamed: {}", b)).unwrap_or_default()
            ),
            LOG_CTX
        );
        let entry = scores
            .entry(key.to_string())
            .or_insert_with(|| (0.0, Vec::new(), None));
        entry.0 += points;
        entry.1.push(evidence.to_string());
        if let Some(b) = blamed {
            entry.2 = Some(b.to_string());
        }
    };

    // --- Evidence from crash report ---

    if let Some(ref exc) = parsed.exception {
        let exc_type = exc.exception_type.to_uppercase();

        // Exception type classification
        if exc_type.contains("ACCESS_VIOLATION")
            || exc_type.contains("SIGSEGV")
            || exc_type.contains("EXC_BAD_ACCESS")
        {
            add_evidence("memory_corruption", 25.0, "exception_access_violation", None);
        }

        if exc_type.contains("OUT_OF_MEMORY")
            || exc_type.contains("SIGKILL")
            || exc_type.contains("HEAP")
        {
            add_evidence("memory_exhaustion", 40.0, "exception_out_of_memory", None);
        }

        // Crash module classification
        if let Some(ref crash_mod) = exc.crash_module {
            if is_plugin(crash_mod) {
                add_evidence("plugin_crash", 40.0, "crash_in_plugin_module", Some(crash_mod));
            } else if is_xplane_core(crash_mod) {
                add_evidence("xplane_bug", 25.0, "crash_in_xplane_core", None);
            } else if is_gpu_driver(crash_mod) {
                add_evidence(
                    "gpu_driver_crash",
                    35.0,
                    "crash_in_gpu_driver",
                    Some(crash_mod),
                );
            } else if is_injected_dll(crash_mod) {
                add_evidence(
                    "dll_conflict",
                    30.0,
                    "crash_in_injected_dll",
                    Some(crash_mod),
                );
            }
        }
    }

    // Stack frame evidence (top 5)
    for frame in parsed.crash_stack.iter().take(5) {
        if let Some(ref mod_name) = frame.module_name {
            if is_plugin(mod_name) {
                add_evidence("plugin_crash", 20.0, "plugin_in_stack_frames", Some(mod_name));
            }
            if is_gpu_driver(mod_name) {
                add_evidence(
                    "gpu_driver_crash",
                    15.0,
                    "gpu_driver_in_stack_frames",
                    Some(mod_name),
                );
            }
        }
    }

    // --- Evidence from log issues ---
    let relevant_categories: &[&str] = &[
        "heavy_memory_pressure",
        "out_of_memory",
        "plugin_error",
        "plugin_assert",
        "vulkan_device_error",
        "vulkan_gfx_error",
        "third_party_blocked",
    ];
    let matched_issues: Vec<&LogIssue> = log_issues
        .iter()
        .filter(|i| relevant_categories.contains(&i.category.as_str()))
        .collect();

    logger::log_info(
        &format!(
            "Cross-referencing with log issues: {} total, {} relevant for crash scoring",
            log_issues.len(),
            matched_issues.len()
        ),
        Some(LOG_CTX),
    );
    for issue in &matched_issues {
        logger::log_info(
            &format!(
                "  Log issue referenced: category={}, severity={}, lines={:?}\n    | {}",
                issue.category,
                issue.severity,
                issue.line_numbers,
                issue.sample_line.replace('\n', "\n    | ")
            ),
            Some(LOG_CTX),
        );
    }

    for issue in log_issues {
        match issue.category.as_str() {
            "heavy_memory_pressure" => {
                add_evidence("memory_exhaustion", 25.0, "log_heavy_memory_pressure", None);
            }
            "out_of_memory" => {
                add_evidence("memory_exhaustion", 35.0, "log_out_of_memory", None);
            }
            "memory_status_critical" => {
                add_evidence("memory_exhaustion", 30.0, "log_memory_status_critical", None);
            }
            "plugin_error" | "plugin_assert" => {
                add_evidence("plugin_crash", 15.0, "log_plugin_error", None);
            }
            "vulkan_device_error" | "vulkan_gfx_error" => {
                add_evidence("gpu_driver_crash", 20.0, "log_vulkan_error", None);
            }
            "third_party_blocked" => {
                add_evidence("dll_conflict", 10.0, "log_third_party_blocked", None);
            }
            _ => {}
        }
    }

    // --- Normalize to percentages ---
    let total: f64 = scores.values().map(|(s, _, _)| s).sum();

    if total <= 0.0 {
        logger::log_info("No crash cause evidence accumulated, no causes to report", Some(LOG_CTX));
        return Vec::new();
    }

    let mut causes: Vec<CrashCause> = scores
        .into_iter()
        .map(|(key, (raw, evidence, blamed))| {
            let pct = (raw / total) * 100.0;
            CrashCause {
                cause_key: key,
                score: (pct * 10.0).round() / 10.0,
                evidence,
                blamed_module: blamed,
            }
        })
        .filter(|c| c.score >= 5.0)
        .collect();

    // Sort descending by score
    causes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Adjust so percentages sum to 100%
    let filtered_sum: f64 = causes.iter().map(|c| c.score).sum();
    if filtered_sum > 0.0 {
        for cause in &mut causes {
            cause.score = ((cause.score / filtered_sum) * 100.0 * 10.0).round() / 10.0;
        }
    }

    // Log final results
    logger::log_info(
        &format!("Crash cause scoring complete, {} cause(s) identified:", causes.len()),
        Some(LOG_CTX),
    );
    for cause in &causes {
        logger::log_info(
            &format!(
                "  - {} {:.1}%{} ({} evidence)",
                cause.cause_key,
                cause.score,
                cause
                    .blamed_module
                    .as_ref()
                    .map(|m| format!(" [{}]", m))
                    .unwrap_or_default(),
                cause.evidence.len()
            ),
            Some(LOG_CTX),
        );
    }

    causes
}

// ========== Main Entry Point ==========

pub async fn analyze_crash_report(
    xplane_path: &str,
    log_issues: &[LogIssue],
    skip_date_check: bool,
) -> Result<Option<DeepCrashAnalysis>, String> {
    logger::log_info("Deep crash analysis started", Some(LOG_CTX));

    let dmp_path = match find_most_recent_crash_report(xplane_path, skip_date_check) {
        Some(p) => p,
        None => {
            logger::log_info("No crash dump files found, skipping deep analysis", Some(LOG_CTX));
            return Ok(None);
        }
    };

    let meta = std::fs::metadata(&dmp_path).map_err(|e| {
        let msg = format!("Cannot read crash report: {}", e);
        logger::log_error(&msg, Some(LOG_CTX));
        msg
    })?;

    let file_size = meta.len();
    logger::log_info(
        &format!(
            "Crash dump file: {} ({:.1} KB)",
            dmp_path.to_string_lossy(),
            file_size as f64 / 1024.0
        ),
        Some(LOG_CTX),
    );

    let timestamp = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let report_info = CrashReportInfo {
        dmp_path: dmp_path.to_string_lossy().to_string(),
        file_name: dmp_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        timestamp,
        file_size,
    };

    match parse_minidump(&dmp_path).await {
        Ok(parsed) => {
            let crash_causes = score_crash_causes(&parsed, log_issues);

            logger::log_info(
                &format!(
                    "Deep crash analysis complete: parse_success=true, {} cause(s), {} warning(s)",
                    crash_causes.len(),
                    parsed.warnings.len()
                ),
                Some(LOG_CTX),
            );

            Ok(Some(DeepCrashAnalysis {
                report_info,
                exception: parsed.exception,
                crash_stack: parsed.crash_stack,
                loaded_modules: parsed.loaded_modules,
                loaded_plugins: parsed.loaded_plugins,
                crash_causes,
                parse_success: true,
                parse_warnings: parsed.warnings,
            }))
        }
        Err(e) => {
            logger::log_error(
                &format!("Minidump parse failed: {}, falling back to log-only scoring", e),
                Some(LOG_CTX),
            );

            let mut warnings = vec![format!("Minidump parse error: {}", e)];
            // Still return partial result with log-based scoring
            let partial_parsed = ParsedMinidump {
                exception: None,
                crash_stack: Vec::new(),
                loaded_modules: Vec::new(),
                loaded_plugins: Vec::new(),
                warnings: Vec::new(),
            };
            let crash_causes = score_crash_causes(&partial_parsed, log_issues);
            warnings.push("Crash causes based on log analysis only".to_string());

            logger::log_info(
                &format!(
                    "Deep crash analysis complete: parse_success=false, {} cause(s) from log only",
                    crash_causes.len()
                ),
                Some(LOG_CTX),
            );

            Ok(Some(DeepCrashAnalysis {
                report_info,
                exception: None,
                crash_stack: Vec::new(),
                loaded_modules: Vec::new(),
                loaded_plugins: Vec::new(),
                crash_causes,
                parse_success: false,
                parse_warnings: warnings,
            }))
        }
    }
}
