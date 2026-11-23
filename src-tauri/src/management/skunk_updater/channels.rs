use anyhow::{anyhow, bail, Context, Result};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::{parse_bool, parse_cfg_lines, resolve_module_url, LOCAL_CFG_FILE};

const V4_STATE_DIR: &str = "skunkcrafts";
const BETA_CFG_FILE: &str = "skunkcrafts_updater_beta.cfg";
const LEGACY_BETA_CFG_FILE: &str = "skunkcraft_updater_beta.cfg";

#[derive(Debug, PartialEq, Eq)]
pub(super) struct LocalConfig {
    pub(super) cfg_path: PathBuf,
    pub(super) version: Option<String>,
    pub(super) liveries: bool,
    pub(super) main_url: String,
    pub(super) main_name: Option<String>,
    pub(super) public_channels: BTreeMap<String, String>,
    pub(super) selected: Option<Channel>,
    pub(super) has_legacy_beta: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Channel {
    pub(super) name: String,
    pub(super) url: String,
    pub(super) public: bool,
}

pub(crate) fn local_config_path(target: &Path) -> Option<PathBuf> {
    let main = target.join(LOCAL_CFG_FILE);
    if main.is_file() {
        return Some(main);
    }
    if let Some(path) = fs::read_dir(target).ok().and_then(|entries| {
        entries.flatten().find_map(|entry| {
            let path = entry.path();
            (entry.file_name().eq_ignore_ascii_case(LOCAL_CFG_FILE) && path.is_file())
                .then_some(path)
        })
    }) {
        return Some(path);
    }
    let early_v4 = target.join(V4_STATE_DIR).join("updater.cfg");
    early_v4.is_file().then_some(early_v4)
}

fn read_cfg_if_present(path: &Path) -> Result<Option<HashMap<String, String>>> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(parse_cfg_lines(&content))),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("Failed to read '{}'", path.display())),
    }
}

fn field<'a>(fields: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    fields
        .get(key)
        .map(String::as_str)
        .filter(|v| !v.is_empty())
}

fn is_main(name: &str) -> bool {
    name.eq_ignore_ascii_case("stable") || name.eq_ignore_ascii_case("Main")
}

pub(super) fn public_channels(fields: &HashMap<String, String>) -> BTreeMap<String, String> {
    fields
        .iter()
        .filter_map(|(key, value)| {
            let name = key.strip_prefix("channel.")?.trim();
            let lower = name.to_ascii_lowercase();
            if name.is_empty()
                || is_main(name)
                || lower.ends_with(".module")
                || lower.ends_with(".zone")
                || super::ensure_http_or_https(value).is_err()
            {
                return None;
            }
            Some((name.to_string(), value.clone()))
        })
        .collect()
}

fn inline_channel<'a>(
    fields: &'a HashMap<String, String>,
    selected: &str,
) -> Option<(&'a str, Option<&'a str>)> {
    fields.iter().find_map(|(key, module)| {
        let name = key.strip_prefix("channel.")?.strip_suffix(".module")?;
        if name != selected {
            return None;
        }
        let zone = field(fields, &format!("channel.{name}.zone"));
        Some((module.as_str(), zone))
    })
}

fn stored_channels(
    state_dir: &Path,
) -> Result<(HashMap<String, HashMap<String, String>>, HashSet<String>)> {
    let mut active = HashMap::new();
    let mut inactive = HashSet::new();
    let entries = match fs::read_dir(state_dir.join("channels")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok((active, inactive)),
        Err(error) => return Err(error).context("Failed to read SkunkCrafts channels"),
    };

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let enabled = name.ends_with(".cfg");
        if !enabled && !name.ends_with(".cfg.removed") && !name.ends_with(".cfg.withdrawn") {
            continue;
        }
        let Some(fields) = read_cfg_if_present(&entry.path())? else {
            continue;
        };
        let Some(channel) = field(&fields, "channel").map(str::to_owned) else {
            continue;
        };
        // The official updater sanitizes filenames independently from the channel name.
        if enabled {
            if active.insert(channel.clone(), fields).is_some() {
                bail!("SkunkCrafts channel '{channel}' has multiple definitions");
            }
        } else {
            inactive.insert(channel);
        }
    }

    Ok((active, inactive))
}

fn tester_channel(
    fields: &HashMap<String, String>,
    default_name: &str,
    main_zone: Option<&str>,
) -> Result<Channel> {
    let module = field(fields, "module").context("SkunkCrafts beta config is missing 'module|'")?;
    Ok(Channel {
        name: field(fields, "channel").unwrap_or(default_name).to_owned(),
        url: resolve_module_url(field(fields, "zone").or(main_zone), module)?,
        public: false,
    })
}

fn tester_enabled(fields: &HashMap<String, String>) -> bool {
    field(fields, "enabled")
        .and_then(parse_bool)
        .unwrap_or(true)
}

fn named_tester_files(target: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(target)? {
        let entry = entry?;
        let filename = entry.file_name();
        let Some(name) = filename
            .to_str()
            .and_then(|filename| filename.strip_prefix("skunkcrafts_updater_beta_"))
            .and_then(|name| name.strip_suffix(".cfg"))
            .filter(|name| !name.is_empty())
        else {
            continue;
        };
        files.push((name.to_owned(), entry.path()));
    }
    Ok(files)
}

fn pending_v4_channel(
    named_testers: &[(String, PathBuf)],
    legacy_beta: Option<&HashMap<String, String>>,
    main_zone: Option<&str>,
    main_url: &str,
    known: &HashMap<String, HashMap<String, String>>,
) -> Result<Option<Channel>> {
    let mut enabled = Vec::new();
    if let Some(fields) = legacy_beta.filter(|fields| tester_enabled(fields)) {
        enabled.push(tester_channel(fields, "beta", main_zone)?);
    }
    for (name, path) in named_testers {
        if let Some(fields) = read_cfg_if_present(path)?.filter(tester_enabled) {
            enabled.push(tester_channel(&fields, name, main_zone)?);
        }
    }
    enabled.retain(|channel| {
        !is_main(&channel.name)
            && !same_depot(&channel.url, main_url)
            && !known.values().any(|fields| {
                field(fields, "module")
                    .and_then(|module| resolve_module_url(field(fields, "zone"), module).ok())
                    .is_some_and(|url| same_depot(&url, &channel.url))
            })
    });
    let Some(first) = enabled.pop() else {
        return Ok(None);
    };
    if enabled
        .iter()
        .any(|channel| !same_depot(&channel.url, &first.url))
    {
        bail!("Open SkunkCrafts Updater to select a channel before updating in XFast Manager");
    }
    Ok(Some(first))
}

pub(super) fn same_depot(first: &str, second: &str) -> bool {
    let Ok(mut first) = reqwest::Url::parse(first) else {
        return false;
    };
    let Ok(mut second) = reqwest::Url::parse(second) else {
        return false;
    };
    first.set_fragment(None);
    second.set_fragment(None);
    first == second
        || (first.origin() == second.origin()
            && first.username() == second.username()
            && first.password() == second.password()
            && first.query() == second.query()
            && first.path().trim_end_matches('/') == second.path().trim_end_matches('/'))
}

pub(super) fn read_local_config(target: &Path, use_beta: Option<bool>) -> Result<LocalConfig> {
    let cfg_path = local_config_path(target)
        .ok_or_else(|| anyhow!("Missing '{}' in '{}'", LOCAL_CFG_FILE, target.display()))?;
    let fields = read_cfg_if_present(&cfg_path)?
        .with_context(|| format!("Missing '{}'", cfg_path.display()))?;
    let main_zone = field(&fields, "zone");
    let main_url = resolve_module_url(
        main_zone,
        field(&fields, "module").context("SkunkCrafts config is missing 'module|'")?,
    )?;
    let state_dir = target.join(V4_STATE_DIR);
    // v4.1+ persists selection separately. See Part 3 of the publishing guide:
    // https://updater.skunkcrafts.com/PUBLISHING.html
    let selection = read_cfg_if_present(&state_dir.join("channel.cfg"))?;
    let inline = field(&fields, "channel");
    let mut public = public_channels(&fields);
    let (stored, inactive) = stored_channels(&state_dir)?;
    for name in &inactive {
        public.remove(name);
    }
    for (name, fields) in &stored {
        if field(fields, "public").and_then(parse_bool) == Some(true) {
            if let Some(url) = field(fields, "module")
                .and_then(|module| resolve_module_url(field(fields, "zone"), module).ok())
            {
                public.entry(name.clone()).or_insert(url);
            }
        }
    }

    let named_testers = if selection.is_none() && inline.is_none() {
        named_tester_files(target)?
    } else {
        Vec::new()
    };
    let v4 = selection.is_some()
        || inline.is_some()
        || state_dir.is_dir()
        || !named_testers.is_empty()
        || fields
            .keys()
            .any(|key| key.starts_with("channel.") && key.ends_with(".module"));
    let legacy_beta = || -> Result<Option<HashMap<String, String>>> {
        let primary = read_cfg_if_present(&target.join(BETA_CFG_FILE))?;
        if primary.is_some() {
            return Ok(primary);
        }
        read_cfg_if_present(&target.join(LEGACY_BETA_CFG_FILE))
    };
    let (selected, has_legacy_beta) = if v4 {
        let selected_name = match &selection {
            Some(fields) => {
                Some(field(fields, "channel").context("Invalid SkunkCrafts channel selection")?)
            }
            None => inline,
        };
        let channel = match selected_name {
            Some(name) if is_main(name) => None,
            Some(name) => {
                let (module, zone, is_public) = if let Some(channel) = stored.get(name) {
                    (
                        field(channel, "module")
                            .context("SkunkCrafts channel is missing 'module|'")?,
                        field(channel, "zone"),
                        field(channel, "public").and_then(parse_bool) == Some(true),
                    )
                } else if inactive.contains(name) {
                    bail!("Selected SkunkCrafts channel '{name}' is no longer available");
                } else if let Some((module, zone)) = inline_channel(&fields, name) {
                    (module, zone, false)
                } else if let Some(module) = public.get(name) {
                    (module.as_str(), None, true)
                } else {
                    bail!("Selected SkunkCrafts channel '{name}' has no depot configuration");
                };
                Some(Channel {
                    name: name.to_owned(),
                    url: resolve_module_url(zone, module)?,
                    public: is_public,
                })
            }
            None => pending_v4_channel(
                &named_testers,
                legacy_beta()?.as_ref(),
                main_zone,
                &main_url,
                &stored,
            )?,
        };
        (channel, false)
    } else if let Some(beta) = legacy_beta()? {
        let selected = if use_beta.unwrap_or_else(|| tester_enabled(&beta)) {
            Some(tester_channel(&beta, "beta", main_zone)?)
        } else {
            None
        };
        (selected, true)
    } else {
        (None, false)
    };

    Ok(LocalConfig {
        cfg_path,
        version: field(&fields, "version").map(str::to_owned),
        liveries: field(&fields, "liveries")
            .and_then(parse_bool)
            .unwrap_or(true),
        main_url,
        main_name: v4.then(|| field(&fields, "main_name").unwrap_or("Main").to_owned()),
        public_channels: public,
        selected,
        has_legacy_beta,
    })
}

pub(super) fn is_protected_path(rel_path: &str) -> bool {
    let mut parts = rel_path.split('/');
    let root = parts.next().unwrap_or_default().to_ascii_lowercase();
    root == V4_STATE_DIR
        || (parts.next().is_none()
            && (matches!(
                root.as_str(),
                LOCAL_CFG_FILE
                    | BETA_CFG_FILE
                    | LEGACY_BETA_CFG_FILE
                    | "skunkcrafts_updater_ignore.txt"
            ) || (root.starts_with("skunkcrafts_updater_beta_") && root.ends_with(".cfg"))))
}
