use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

const METERS_TO_FEET: f64 = 3.28084;
const MPS_TO_KNOTS: f64 = 1.94384;

const STREAM_EVENT_STATE: &str = "map-plane-state";
const STREAM_EVENT_CONNECTION: &str = "map-plane-connection";

const DF_LATITUDE: &str = "sim/flightmodel/position/latitude";
const DF_LONGITUDE: &str = "sim/flightmodel/position/longitude";
const DF_ALTITUDE_MSL: &str = "sim/flightmodel/position/elevation";
const DF_ALTITUDE_AGL: &str = "sim/flightmodel/position/y_agl";
const DF_HEADING: &str = "sim/flightmodel/position/psi";
const DF_GROUNDSPEED: &str = "sim/flightmodel/position/groundspeed";
const DF_INDICATED_AIRSPEED: &str = "sim/flightmodel/position/indicated_airspeed";
const DF_VERTICAL_SPEED: &str = "sim/cockpit2/gauges/indicators/vvi_fpm_pilot";

const MAP_DATAREFS: [&str; 8] = [
    DF_LATITUDE,
    DF_LONGITUDE,
    DF_ALTITUDE_MSL,
    DF_ALTITUDE_AGL,
    DF_HEADING,
    DF_GROUNDSPEED,
    DF_INDICATED_AIRSPEED,
    DF_VERTICAL_SPEED,
];

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent("XFast Manager Map")
        .timeout(Duration::from_secs(15))
        .build()
        .expect("failed to create reqwest client")
});

static MAP_INDEX_STATE: LazyLock<RwLock<MapIndexState>> =
    LazyLock::new(|| RwLock::new(MapIndexState::default()));

static PLANE_STREAM_STATE: LazyLock<Arc<Mutex<PlaneStreamController>>> =
    LazyLock::new(|| Arc::new(Mutex::new(PlaneStreamController::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapBounds {
    pub north: f64,
    pub south: f64,
    pub east: f64,
    pub west: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAirport {
    pub icao: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub airport_type: String,
    pub is_custom: bool,
    pub elevation: Option<f64>,
    pub runway_count: Option<u32>,
    pub surface_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MapAirportDetail {
    pub icao: String,
    pub name: String,
    pub airport_type: String,
    pub is_custom: bool,
    pub runways: Vec<MapAirportDetailRunway>,
    pub helipads: Vec<MapAirportDetailHelipad>,
    pub gates: Vec<MapAirportDetailGate>,
    pub tower: Option<MapAirportDetailTower>,
    pub beacon: Option<MapAirportDetailBeacon>,
    pub windsocks: Vec<MapAirportDetailWindsock>,
    pub signs: Vec<MapAirportDetailSign>,
    pub taxiways: Vec<MapAirportDetailTaxiway>,
    pub pavements: Vec<MapPavement>,
    pub linear_features: Vec<MapLinearFeature>,
    pub boundaries: Vec<MapBoundary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAirportDetailRunway {
    pub name: String,
    pub width_m: Option<f64>,
    pub surface_code: Option<i32>,
    pub surface_type: Option<String>,
    pub shoulder_surface_code: Option<i32>,
    pub shoulder_surface_type: Option<String>,
    pub shoulder_width_m: Option<f64>,
    pub centerline_lights: bool,
    pub edge_lights: bool,
    pub end1_name: String,
    pub end1_lat: f64,
    pub end1_lon: f64,
    pub end1_marking: Option<i32>,
    pub end1_lighting: Option<i32>,
    pub end1_tdz_lighting: bool,
    pub end1_reil: Option<i32>,
    pub end2_name: String,
    pub end2_lat: f64,
    pub end2_lon: f64,
    pub end2_marking: Option<i32>,
    pub end2_lighting: Option<i32>,
    pub end2_tdz_lighting: bool,
    pub end2_reil: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAirportDetailHelipad {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub heading: Option<f64>,
    pub length_m: Option<f64>,
    pub width_m: Option<f64>,
    pub surface_code: Option<i32>,
    pub surface_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAirportDetailGate {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub heading: Option<f64>,
    pub location_type: Option<String>,
    pub operation_type: Option<String>,
    pub width_code: Option<String>,
    pub airlines: Vec<String>,
    pub is_legacy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAirportDetailTower {
    pub lat: f64,
    pub lon: f64,
    pub height_m: Option<f64>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAirportDetailBeacon {
    pub lat: f64,
    pub lon: f64,
    pub beacon_type: Option<i32>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAirportDetailWindsock {
    pub lat: f64,
    pub lon: f64,
    pub illuminated: bool,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAirportDetailSign {
    pub lat: f64,
    pub lon: f64,
    pub heading: Option<f64>,
    pub size: Option<i32>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAirportDetailTaxiway {
    pub name: String,
    pub from_lat: f64,
    pub from_lon: f64,
    pub to_lat: f64,
    pub to_lon: f64,
}

/// A pavement / apron polygon parsed from apt.dat row codes 110 + 111-116
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapPavement {
    pub surface_type: i32,
    pub smoothness: f64,
    pub texture_orientation: f64,
    pub name: String,
    /// Outer ring as [[lon, lat], ...]
    pub coordinates: Vec<[f64; 2]>,
    /// Hole rings, each as [[lon, lat], ...]
    pub holes: Vec<Vec<[f64; 2]>>,
}

/// A linear feature (painted line / taxiway marking) from apt.dat row code 120 + 111-116
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapLinearFeature {
    pub name: String,
    pub line_type: i32,
    pub light_type: i32,
    /// Coordinates as [[lon, lat], ...]
    pub coordinates: Vec<[f64; 2]>,
}

/// Airport boundary from apt.dat row code 130 + 111-116
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapBoundary {
    /// Outer ring as [[lon, lat], ...]
    pub coordinates: Vec<[f64; 2]>,
    /// Hole rings
    pub holes: Vec<Vec<[f64; 2]>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapProcedureWaypoint {
    pub fix_id: String,
    pub fix_region: String,
    pub fix_type: String,
    pub path_terminator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapProcedure {
    pub procedure_type: String,
    pub name: String,
    pub runway: Option<String>,
    pub transition: Option<String>,
    pub waypoint_count: usize,
    pub waypoints: Vec<MapProcedureWaypoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MapAirportProcedures {
    pub icao: String,
    pub sids: Vec<MapProcedure>,
    pub stars: Vec<MapProcedure>,
    pub approaches: Vec<MapProcedure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapNavaid {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub navaid_type: String,
    pub frequency: Option<f64>,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapWaypoint {
    pub id: String,
    pub region: Option<String>,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAirwaySegment {
    pub name: String,
    pub from_id: String,
    pub to_id: String,
    pub from_lat: f64,
    pub from_lon: f64,
    pub to_lat: f64,
    pub to_lon: f64,
    pub is_high: bool,
    pub base_fl: Option<i32>,
    pub top_fl: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapIls {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub course: Option<f64>,
    pub airport: Option<String>,
    pub runway: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAirspace {
    pub name: String,
    pub class_code: String,
    pub upper_limit: Option<String>,
    pub lower_limit: Option<String>,
    pub coordinates: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MapNavSnapshot {
    pub navaids: Vec<MapNavaid>,
    pub waypoints: Vec<MapWaypoint>,
    pub airways: Vec<MapAirwaySegment>,
    pub ils: Vec<MapIls>,
    pub airspaces: Vec<MapAirspace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapLayerRequest {
    pub lat: f64,
    pub lon: f64,
    pub radius_nm: f64,
    pub include_navaids: Option<bool>,
    pub include_waypoints: Option<bool>,
    pub include_airways: Option<bool>,
    pub include_ils: Option<bool>,
    pub include_airspaces: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MapDataStatus {
    pub loaded: bool,
    pub xplane_path: Option<String>,
    pub airport_count: usize,
    pub navaid_count: usize,
    pub waypoint_count: usize,
    pub airway_count: usize,
    pub ils_count: usize,
    pub airspace_count: usize,
    pub last_loaded_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MapPlaneState {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(rename = "altitudeMSL")]
    pub altitude_msl: Option<f64>,
    #[serde(rename = "altitudeAGL")]
    pub altitude_agl: Option<f64>,
    pub heading: Option<f64>,
    pub groundspeed: Option<f64>,
    pub indicated_airspeed: Option<f64>,
    pub vertical_speed: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapPlaneStreamStatus {
    pub running: bool,
    pub connected: bool,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatarefCatalogEntry {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatarefCatalogResponse {
    pub data: Option<Vec<DatarefCatalogEntry>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatarefValueResponse {
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Default)]
struct MapIndexState {
    status: MapDataStatus,
    airports: Vec<MapAirport>,
    airport_sources: HashMap<String, MapAirportSource>,
    airport_details: HashMap<String, MapAirportDetail>,
    airport_procedures: HashMap<String, MapAirportProcedures>,
    navaids: Vec<MapNavaid>,
    waypoints: Vec<MapWaypoint>,
    airways: Vec<MapAirwaySegment>,
    ils: Vec<MapIls>,
    airspaces: Vec<MapAirspace>,
}

#[derive(Debug)]
struct ParsedIndexData {
    airports: Vec<MapAirport>,
    airport_sources: HashMap<String, MapAirportSource>,
    navaids: Vec<MapNavaid>,
    waypoints: Vec<MapWaypoint>,
    airways: Vec<MapAirwaySegment>,
    ils: Vec<MapIls>,
    airspaces: Vec<MapAirspace>,
}

#[derive(Debug, Clone)]
struct MapAirportSource {
    apt_path: String,
    is_custom: bool,
}

#[derive(Debug)]
struct PlaneStreamController {
    running: bool,
    connected: bool,
    port: u16,
    stop_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl Default for PlaneStreamController {
    fn default() -> Self {
        Self {
            running: false,
            connected: false,
            port: 8086,
            stop_tx: None,
        }
    }
}

#[derive(Debug, Clone)]
struct AirportBuilder {
    icao: String,
    name: String,
    airport_type: String,
    elevation: Option<f64>,
    datum_lat: Option<f64>,
    datum_lon: Option<f64>,
    fallback_lat: Option<f64>,
    fallback_lon: Option<f64>,
    runway_count: u32,
    primary_surface_code: Option<i32>,
    is_custom: bool,
}

#[derive(Debug, Clone)]
struct AirportDetailBuilder {
    icao: String,
    name: String,
    airport_type: String,
    is_custom: bool,
    runways: Vec<MapAirportDetailRunway>,
    helipads: Vec<MapAirportDetailHelipad>,
    gates: Vec<MapAirportDetailGate>,
    tower: Option<MapAirportDetailTower>,
    beacon: Option<MapAirportDetailBeacon>,
    windsocks: Vec<MapAirportDetailWindsock>,
    signs: Vec<MapAirportDetailSign>,
    taxi_nodes: HashMap<i32, [f64; 2]>,
    taxi_edges: Vec<(i32, i32, String)>,
    pavements: Vec<MapPavement>,
    linear_features: Vec<MapLinearFeature>,
    boundaries: Vec<MapBoundary>,
}

impl AirportBuilder {
    fn finalize(self) -> Option<MapAirport> {
        let lat = self.datum_lat.or(self.fallback_lat)?;
        let lon = self.datum_lon.or(self.fallback_lon)?;

        if !is_valid_lat_lon(lat, lon) {
            return None;
        }

        Some(MapAirport {
            icao: self.icao,
            name: self.name,
            lat,
            lon,
            airport_type: self.airport_type,
            is_custom: self.is_custom,
            elevation: self.elevation,
            runway_count: Some(self.runway_count).filter(|v| *v > 0),
            surface_type: self.primary_surface_code.and_then(surface_code_to_name),
        })
    }
}

impl AirportDetailBuilder {
    fn from_header(header: AirportBuilder) -> Self {
        Self {
            icao: header.icao,
            name: header.name,
            airport_type: header.airport_type,
            is_custom: header.is_custom,
            runways: Vec::new(),
            helipads: Vec::new(),
            gates: Vec::new(),
            tower: None,
            beacon: None,
            windsocks: Vec::new(),
            signs: Vec::new(),
            taxi_nodes: HashMap::new(),
            taxi_edges: Vec::new(),
            pavements: Vec::new(),
            linear_features: Vec::new(),
            boundaries: Vec::new(),
        }
    }

    fn finalize(self) -> MapAirportDetail {
        let AirportDetailBuilder {
            icao,
            name,
            airport_type,
            is_custom,
            runways,
            helipads,
            gates,
            tower,
            beacon,
            windsocks,
            signs,
            taxi_nodes,
            taxi_edges,
            pavements,
            linear_features,
            boundaries,
        } = self;

        let mut taxiways = Vec::with_capacity(taxi_edges.len());
        for (from_id, to_id, edge_name) in taxi_edges {
            let Some(from) = taxi_nodes.get(&from_id) else {
                continue;
            };
            let Some(to) = taxi_nodes.get(&to_id) else {
                continue;
            };
            taxiways.push(MapAirportDetailTaxiway {
                name: edge_name,
                from_lat: from[0],
                from_lon: from[1],
                to_lat: to[0],
                to_lon: to[1],
            });
        }

        MapAirportDetail {
            icao,
            name,
            airport_type,
            is_custom,
            runways,
            helipads,
            gates,
            tower,
            beacon,
            windsocks,
            signs,
            taxiways,
            pavements,
            linear_features,
            boundaries,
        }
    }
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as i64
}

fn path_exists(path: &Path) -> bool {
    std::fs::metadata(path).is_ok()
}

fn resolve_data_path(xplane_path: &Path, default_relative: &str, custom_relative: &str) -> PathBuf {
    let custom = xplane_path.join(custom_relative);
    if path_exists(&custom) {
        custom
    } else {
        xplane_path.join(default_relative)
    }
}

fn global_apt_path(xplane_path: &Path) -> PathBuf {
    xplane_path
        .join("Global Scenery")
        .join("Global Airports")
        .join("Earth nav data")
        .join("apt.dat")
}

fn custom_scenery_apt_paths(xplane_path: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let root = xplane_path.join("Custom Scenery");
    let entries = match std::fs::read_dir(root) {
        Ok(v) => v,
        Err(_) => return out,
    };

    for entry in entries.flatten() {
        let path = entry.path().join("Earth nav data").join("apt.dat");
        if path_exists(&path) {
            out.push(path);
        }
    }

    out.sort();
    out
}

fn build_data_index_sync(xplane_path: &str) -> Result<ParsedIndexData, String> {
    let root = PathBuf::from(xplane_path);
    if !path_exists(&root) {
        return Err(format!("X-Plane path does not exist: {}", xplane_path));
    }

    let mut airports_by_icao: HashMap<String, MapAirport> = HashMap::new();
    let mut airport_sources: HashMap<String, MapAirportSource> = HashMap::new();

    let global_path = global_apt_path(&root);
    if path_exists(&global_path) {
        parse_apt_file(
            &global_path,
            false,
            &mut airports_by_icao,
            &mut airport_sources,
        )?;
    }

    for custom_path in custom_scenery_apt_paths(&root) {
        parse_apt_file(
            &custom_path,
            true,
            &mut airports_by_icao,
            &mut airport_sources,
        )?;
    }

    let nav_path = resolve_data_path(
        &root,
        "Resources/default data/earth_nav.dat",
        "Custom Data/earth_nav.dat",
    );
    let fix_path = resolve_data_path(
        &root,
        "Resources/default data/earth_fix.dat",
        "Custom Data/earth_fix.dat",
    );
    let awy_path = resolve_data_path(
        &root,
        "Resources/default data/earth_awy.dat",
        "Custom Data/earth_awy.dat",
    );
    let airspace_path = resolve_data_path(
        &root,
        "Resources/default data/airspaces/airspace.txt",
        "Custom Data/airspaces/airspace.txt",
    );

    let (navaids, ils, navaid_lookup) = parse_nav_file(&nav_path)?;
    let (waypoints, waypoint_lookup) = parse_fix_file(&fix_path)?;
    let airways = parse_awy_file(&awy_path, &navaid_lookup, &waypoint_lookup)?;
    let airspaces = parse_airspace_file(&airspace_path)?;

    let mut airports: Vec<MapAirport> = airports_by_icao.into_values().collect();
    airports.sort_by(|a, b| a.icao.cmp(&b.icao));

    Ok(ParsedIndexData {
        airports,
        airport_sources,
        navaids,
        waypoints,
        airways,
        ils,
        airspaces,
    })
}

fn parse_apt_file(
    path: &Path,
    is_custom: bool,
    airports: &mut HashMap<String, MapAirport>,
    airport_sources: &mut HashMap<String, MapAirportSource>,
) -> Result<(), String> {
    let file = File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);
    let source_path = path.to_string_lossy().to_string();

    let mut current: Option<AirportBuilder> = None;

    for line in reader.lines() {
        let line = match line {
            Ok(v) => v,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "99" {
            if let Some(builder) = current.take() {
                if let Some(airport) = builder.finalize() {
                    let icao = airport.icao.clone();
                    airports.insert(icao.clone(), airport);
                    airport_sources.insert(
                        icao,
                        MapAirportSource {
                            apt_path: source_path.clone(),
                            is_custom,
                        },
                    );
                }
            }
            break;
        }

        if is_airport_header(trimmed) {
            if let Some(builder) = current.take() {
                if let Some(airport) = builder.finalize() {
                    let icao = airport.icao.clone();
                    airports.insert(icao.clone(), airport);
                    airport_sources.insert(
                        icao,
                        MapAirportSource {
                            apt_path: source_path.clone(),
                            is_custom,
                        },
                    );
                }
            }
            current = parse_airport_header(trimmed, is_custom);
            continue;
        }

        let Some(airport) = current.as_mut() else {
            continue;
        };

        if let Some(rest) = trimmed.strip_prefix("1302 ") {
            parse_airport_metadata(rest, airport);
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("100 ") {
            parse_airport_runway(rest, airport);
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("101 ") {
            parse_airport_water_runway(rest, airport);
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("102 ") {
            parse_airport_helipad(rest, airport);
            continue;
        }
    }

    if let Some(builder) = current.take() {
        if let Some(airport) = builder.finalize() {
            let icao = airport.icao.clone();
            airports.insert(icao.clone(), airport);
            airport_sources.insert(
                icao,
                MapAirportSource {
                    apt_path: source_path,
                    is_custom,
                },
            );
        }
    }

    Ok(())
}

fn is_airport_header(line: &str) -> bool {
    line.starts_with("1 ") || line.starts_with("16 ") || line.starts_with("17 ")
}

fn parse_airport_header(line: &str, is_custom: bool) -> Option<AirportBuilder> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 6 {
        return None;
    }

    let airport_type = match parts[0] {
        "1" => "land",
        "16" => "seaplane",
        "17" => "heliport",
        _ => "unknown",
    }
    .to_string();

    let icao = parts[4].trim().to_uppercase();
    if icao.len() < 2 || icao.len() > 8 {
        return None;
    }

    let name = parts[5..].join(" ");
    if name.is_empty() {
        return None;
    }

    let elevation = parts[1].parse::<f64>().ok();

    Some(AirportBuilder {
        icao,
        name,
        airport_type,
        elevation,
        datum_lat: None,
        datum_lon: None,
        fallback_lat: None,
        fallback_lon: None,
        runway_count: 0,
        primary_surface_code: None,
        is_custom,
    })
}

fn parse_airport_metadata(rest: &str, airport: &mut AirportBuilder) {
    let mut iter = rest.split_whitespace();
    let key = match iter.next() {
        Some(v) => v,
        None => return,
    };
    let value = iter.collect::<Vec<&str>>().join(" ");
    if value.is_empty() {
        return;
    }

    match key {
        "datum_lat" => {
            if let Ok(v) = value.parse::<f64>() {
                airport.datum_lat = Some(v);
            }
        }
        "datum_lon" => {
            if let Ok(v) = value.parse::<f64>() {
                airport.datum_lon = Some(v);
            }
        }
        _ => {}
    }
}

fn parse_airport_runway(rest: &str, airport: &mut AirportBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 11 {
        return;
    }

    airport.runway_count = airport.runway_count.saturating_add(1);

    if let Ok(surface) = parts[1].parse::<i32>() {
        match airport.primary_surface_code {
            Some(existing) if existing <= surface => {}
            _ => {
                airport.primary_surface_code = Some(surface);
            }
        }
    }

    if airport.fallback_lat.is_none() || airport.fallback_lon.is_none() {
        if let (Ok(lat), Ok(lon)) = (parts[8].parse::<f64>(), parts[9].parse::<f64>()) {
            if is_valid_lat_lon(lat, lon) {
                airport.fallback_lat = Some(lat);
                airport.fallback_lon = Some(lon);
            }
        }
    }
}

fn parse_airport_water_runway(rest: &str, airport: &mut AirportBuilder) {
    if airport.fallback_lat.is_some() && airport.fallback_lon.is_some() {
        return;
    }

    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 5 {
        return;
    }

    if let (Ok(lat), Ok(lon)) = (parts[3].parse::<f64>(), parts[4].parse::<f64>()) {
        if is_valid_lat_lon(lat, lon) {
            airport.fallback_lat = Some(lat);
            airport.fallback_lon = Some(lon);
        }
    }
}

fn parse_airport_helipad(rest: &str, airport: &mut AirportBuilder) {
    if airport.fallback_lat.is_some() && airport.fallback_lon.is_some() {
        return;
    }

    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 3 {
        return;
    }

    if let (Ok(lat), Ok(lon)) = (parts[1].parse::<f64>(), parts[2].parse::<f64>()) {
        if is_valid_lat_lon(lat, lon) {
            airport.fallback_lat = Some(lat);
            airport.fallback_lon = Some(lon);
        }
    }
}

fn parse_airport_detail_runway(rest: &str, airport: &mut AirportDetailBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 19 {
        return;
    }

    let end1_name = parts[7].trim().to_uppercase();
    let end2_name = parts[16].trim().to_uppercase();

    let (end1_lat, end1_lon) = match (parts[8].parse::<f64>(), parts[9].parse::<f64>()) {
        (Ok(lat), Ok(lon)) if is_valid_lat_lon(lat, lon) => (lat, lon),
        _ => return,
    };

    let (end2_lat, end2_lon) = match (parts[17].parse::<f64>(), parts[18].parse::<f64>()) {
        (Ok(lat), Ok(lon)) if is_valid_lat_lon(lat, lon) => (lat, lon),
        _ => return,
    };

    let name = if !end1_name.is_empty() && !end2_name.is_empty() {
        format!("{}/{}", end1_name, end2_name)
    } else if !end1_name.is_empty() {
        end1_name.clone()
    } else if !end2_name.is_empty() {
        end2_name.clone()
    } else {
        format!("RWY {}", airport.runways.len() + 1)
    };

    let surface_code = parts[1].parse::<i32>().ok();
    let width_m = parts[0]
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0);
    let centerline_lights = parts
        .get(4)
        .and_then(|v| v.parse::<i32>().ok())
        .map(|v| v > 0)
        .unwrap_or(false);
    let edge_lights = parts
        .get(5)
        .and_then(|v| v.parse::<i32>().ok())
        .map(|v| v > 0)
        .unwrap_or(false);

    let shoulder_token = parts.get(2).and_then(|v| v.parse::<i32>().ok());
    let (mut shoulder_surface_code, mut shoulder_width_m) = match shoulder_token {
        Some(v) if v >= 100 => (Some(v % 100), Some((v / 100) as f64)),
        Some(v) => (Some(v), None),
        None => (None, None),
    };

    shoulder_surface_code = shoulder_surface_code.filter(|v| *v > 0);
    shoulder_width_m = shoulder_width_m.filter(|v| v.is_finite() && *v > 0.0);

    let end1_marking = parts.get(12).and_then(|v| v.parse::<i32>().ok());
    let end1_lighting = parts.get(13).and_then(|v| v.parse::<i32>().ok());
    let end1_tdz_lighting = parts
        .get(14)
        .and_then(|v| v.parse::<i32>().ok())
        .map(|v| v > 0)
        .unwrap_or(false);
    let end1_reil = parts.get(15).and_then(|v| v.parse::<i32>().ok());

    let end2_marking = parts.get(21).and_then(|v| v.parse::<i32>().ok());
    let end2_lighting = parts.get(22).and_then(|v| v.parse::<i32>().ok());
    let end2_tdz_lighting = parts
        .get(23)
        .and_then(|v| v.parse::<i32>().ok())
        .map(|v| v > 0)
        .unwrap_or(false);
    let end2_reil = parts.get(24).and_then(|v| v.parse::<i32>().ok());

    airport.runways.push(MapAirportDetailRunway {
        name,
        width_m,
        surface_code,
        surface_type: surface_code.and_then(surface_code_to_name),
        shoulder_surface_code,
        shoulder_surface_type: shoulder_surface_code.and_then(surface_code_to_name),
        shoulder_width_m,
        centerline_lights,
        edge_lights,
        end1_name,
        end1_lat,
        end1_lon,
        end1_marking,
        end1_lighting,
        end1_tdz_lighting,
        end1_reil,
        end2_name,
        end2_lat,
        end2_lon,
        end2_marking,
        end2_lighting,
        end2_tdz_lighting,
        end2_reil,
    });
}

fn parse_airport_detail_water_runway(rest: &str, airport: &mut AirportDetailBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 8 {
        return;
    }

    let end1_name = parts[2].trim().to_uppercase();
    let end2_name = parts[5].trim().to_uppercase();

    let (end1_lat, end1_lon) = match (parts[3].parse::<f64>(), parts[4].parse::<f64>()) {
        (Ok(lat), Ok(lon)) if is_valid_lat_lon(lat, lon) => (lat, lon),
        _ => return,
    };
    let (end2_lat, end2_lon) = match (parts[6].parse::<f64>(), parts[7].parse::<f64>()) {
        (Ok(lat), Ok(lon)) if is_valid_lat_lon(lat, lon) => (lat, lon),
        _ => return,
    };

    let name = if !end1_name.is_empty() && !end2_name.is_empty() {
        format!("{}/{}", end1_name, end2_name)
    } else if !end1_name.is_empty() {
        end1_name.clone()
    } else if !end2_name.is_empty() {
        end2_name.clone()
    } else {
        format!("WTR {}", airport.runways.len() + 1)
    };

    airport.runways.push(MapAirportDetailRunway {
        name,
        width_m: parts[0]
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v > 0.0),
        surface_code: Some(13),
        surface_type: surface_code_to_name(13),
        shoulder_surface_code: None,
        shoulder_surface_type: None,
        shoulder_width_m: None,
        centerline_lights: false,
        edge_lights: false,
        end1_name,
        end1_lat,
        end1_lon,
        end1_marking: None,
        end1_lighting: None,
        end1_tdz_lighting: false,
        end1_reil: None,
        end2_name,
        end2_lat,
        end2_lon,
        end2_marking: None,
        end2_lighting: None,
        end2_tdz_lighting: false,
        end2_reil: None,
    });
}

fn parse_airport_detail_helipad(rest: &str, airport: &mut AirportDetailBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 3 {
        return;
    }

    let (lat, lon) = match (parts[1].parse::<f64>(), parts[2].parse::<f64>()) {
        (Ok(lat), Ok(lon)) if is_valid_lat_lon(lat, lon) => (lat, lon),
        _ => return,
    };

    let name = if parts[0].trim().is_empty() {
        format!("H{}", airport.helipads.len() + 1)
    } else {
        parts[0].trim().to_uppercase()
    };

    let heading = parts
        .get(3)
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v.rem_euclid(360.0));
    let length_m = parts
        .get(4)
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.0);
    let width_m = parts
        .get(5)
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.0);
    let surface_code = parts.get(6).and_then(|v| v.parse::<i32>().ok());

    airport.helipads.push(MapAirportDetailHelipad {
        name,
        lat,
        lon,
        heading,
        length_m,
        width_m,
        surface_code,
        surface_type: surface_code.and_then(surface_code_to_name),
    });
}

fn parse_airport_detail_start_legacy(rest: &str, airport: &mut AirportDetailBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 3 {
        return;
    }

    let (lat, lon) = match (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
        (Ok(lat), Ok(lon)) if is_valid_lat_lon(lat, lon) => (lat, lon),
        _ => return,
    };

    let heading = parts[2].parse::<f64>().ok().map(|v| v.rem_euclid(360.0));
    let name = {
        let n = parts[3..].join(" ").trim().to_string();
        if n.is_empty() {
            format!("Ramp {}", airport.gates.len() + 1)
        } else {
            n
        }
    };

    airport.gates.push(MapAirportDetailGate {
        name,
        lat,
        lon,
        heading,
        location_type: Some("legacy".to_string()),
        operation_type: None,
        width_code: None,
        airlines: Vec::new(),
        is_legacy: true,
    });
}

fn parse_airport_detail_start_new(rest: &str, airport: &mut AirportDetailBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let (lat, lon) = match (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
        (Ok(lat), Ok(lon)) if is_valid_lat_lon(lat, lon) => (lat, lon),
        _ => return,
    };

    let heading = parts
        .get(2)
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v.rem_euclid(360.0));
    let location_type = parts
        .get(3)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let name = if parts.len() > 5 {
        let n = parts[5..].join(" ").trim().to_string();
        if n.is_empty() {
            format!("Ramp {}", airport.gates.len() + 1)
        } else {
            n
        }
    } else {
        format!("Ramp {}", airport.gates.len() + 1)
    };

    airport.gates.push(MapAirportDetailGate {
        name,
        lat,
        lon,
        heading,
        location_type,
        operation_type: None,
        width_code: None,
        airlines: Vec::new(),
        is_legacy: false,
    });
}

fn parse_airport_detail_start_metadata(rest: &str, airport: &mut AirportDetailBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    let Some(last_gate) = airport.gates.last_mut() else {
        return;
    };

    last_gate.width_code = parts
        .first()
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    last_gate.operation_type = parts
        .get(1)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    if parts.len() > 2 {
        last_gate.airlines = parts[2..]
            .iter()
            .map(|v| v.trim().to_uppercase())
            .filter(|v| !v.is_empty())
            .collect();
    }
}

fn parse_airport_detail_tower(rest: &str, airport: &mut AirportDetailBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let (lat, lon) = match (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
        (Ok(lat), Ok(lon)) if is_valid_lat_lon(lat, lon) => (lat, lon),
        _ => return,
    };

    let height_m = parts
        .get(2)
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v >= 0.0);
    let name = if parts.len() > 3 {
        let text = parts[3..].join(" ").trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    } else {
        None
    };

    airport.tower = Some(MapAirportDetailTower {
        lat,
        lon,
        height_m,
        name,
    });
}

fn parse_airport_detail_beacon(rest: &str, airport: &mut AirportDetailBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let (lat, lon) = match (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
        (Ok(lat), Ok(lon)) if is_valid_lat_lon(lat, lon) => (lat, lon),
        _ => return,
    };

    let beacon_type = parts.get(2).and_then(|v| v.parse::<i32>().ok());
    let name = if parts.len() > 3 {
        let text = parts[3..].join(" ").trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    } else {
        None
    };

    airport.beacon = Some(MapAirportDetailBeacon {
        lat,
        lon,
        beacon_type,
        name,
    });
}

fn parse_airport_detail_windsock(rest: &str, airport: &mut AirportDetailBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let (lat, lon) = match (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
        (Ok(lat), Ok(lon)) if is_valid_lat_lon(lat, lon) => (lat, lon),
        _ => return,
    };

    let illuminated = parts
        .get(2)
        .and_then(|v| v.parse::<i32>().ok())
        .map(|v| v > 0)
        .unwrap_or(false);

    let name = if parts.len() > 3 {
        let text = parts[3..].join(" ").trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    } else {
        None
    };

    airport.windsocks.push(MapAirportDetailWindsock {
        lat,
        lon,
        illuminated,
        name,
    });
}

fn parse_airport_detail_sign(rest: &str, airport: &mut AirportDetailBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 6 {
        return;
    }

    let (lat, lon) = match (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
        (Ok(lat), Ok(lon)) if is_valid_lat_lon(lat, lon) => (lat, lon),
        _ => return,
    };

    let heading = parts
        .get(2)
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v.rem_euclid(360.0));
    let size = parts.get(4).and_then(|v| v.parse::<i32>().ok());
    let text = parts[5..].join(" ").trim().to_string();
    if text.is_empty() {
        return;
    }

    airport.signs.push(MapAirportDetailSign {
        lat,
        lon,
        heading,
        size,
        text,
    });
}

fn parse_airport_detail_taxi_node(rest: &str, airport: &mut AirportDetailBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let (lat, lon) = match (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
        (Ok(lat), Ok(lon)) if is_valid_lat_lon(lat, lon) => (lat, lon),
        _ => return,
    };

    let node_id = parts.iter().rev().find_map(|v| v.parse::<i32>().ok());
    let Some(node_id) = node_id else {
        return;
    };

    airport.taxi_nodes.insert(node_id, [lat, lon]);
}

fn parse_airport_detail_taxi_edge(rest: &str, airport: &mut AirportDetailBuilder) {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let (from_id, to_id) = match (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
        (Ok(from_id), Ok(to_id)) => (from_id, to_id),
        _ => return,
    };

    let restriction = parts
        .get(3)
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();
    if restriction == "runway" {
        return;
    }

    let name = if parts.len() > 4 {
        let text = parts[4..].join(" ").trim().to_string();
        if text.is_empty() {
            format!("TAXI {}-{}", from_id, to_id)
        } else {
            text
        }
    } else {
        format!("TAXI {}-{}", from_id, to_id)
    };

    airport.taxi_edges.push((from_id, to_id, name));
}

// ---------------------------------------------------------------------------
// Bezier curve helpers for apt.dat geometry (row codes 111-116)
// ---------------------------------------------------------------------------

const BEZIER_RESOLUTION: usize = 20;

/// Quadratic bezier: B(t) = (1-t)^2 * P0 + 2(1-t)t * P1 + t^2 * P2
fn bezier_quadratic(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], steps: usize) -> Vec<[f64; 2]> {
    let mut pts = Vec::with_capacity(steps);
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let u = 1.0 - t;
        let lon = u * u * p0[0] + 2.0 * u * t * p1[0] + t * t * p2[0];
        let lat = u * u * p0[1] + 2.0 * u * t * p1[1] + t * t * p2[1];
        pts.push([lon, lat]);
    }
    pts
}

/// Cubic bezier: B(t) = (1-t)^3 * P0 + 3(1-t)^2 t * P1 + 3(1-t)t^2 * P2 + t^3 * P3
fn bezier_cubic(
    p0: [f64; 2],
    p1: [f64; 2],
    p2: [f64; 2],
    p3: [f64; 2],
    steps: usize,
) -> Vec<[f64; 2]> {
    let mut pts = Vec::with_capacity(steps);
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let u = 1.0 - t;
        let lon = u * u * u * p0[0]
            + 3.0 * u * u * t * p1[0]
            + 3.0 * u * t * t * p2[0]
            + t * t * t * p3[0];
        let lat = u * u * u * p0[1]
            + 3.0 * u * u * t * p1[1]
            + 3.0 * u * t * t * p2[1]
            + t * t * t * p3[1];
        pts.push([lon, lat]);
    }
    pts
}

/// Mirror a control point through its vertex: reflected = 2*vertex - control
fn mirror_control(vertex: [f64; 2], control: [f64; 2]) -> [f64; 2] {
    [2.0 * vertex[0] - control[0], 2.0 * vertex[1] - control[1]]
}

/// Represents a parsed node from row codes 111-116
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AptNode {
    lat: f64,
    lon: f64,
    /// Bezier control point (for row codes 112/114/116)
    ctrl_lat: Option<f64>,
    ctrl_lon: Option<f64>,
    /// Painted line type (from optional fields)
    line_type: i32,
    /// Lighting type (from optional fields)
    light_type: i32,
    /// Whether this node closes a ring (113/114)
    closes_ring: bool,
    /// Whether this node ends the path (115/116)
    ends_path: bool,
}

/// A single ring/path extracted from a sequence of apt.dat nodes
#[derive(Debug, Clone)]
struct ParsedRing {
    /// Coordinates as [lon, lat] pairs (GeoJSON order)
    coords: Vec<[f64; 2]>,
    /// Per-node line type info (parallel to the original nodes)
    node_line_types: Vec<(i32, i32)>,
    /// Whether this ring was detected as a hole (clockwise winding)
    is_hole: bool,
}

/// Check if a ring has clockwise winding (= hole in GeoJSON convention)
fn is_clockwise(coords: &[[f64; 2]]) -> bool {
    if coords.len() < 3 {
        return false;
    }
    let mut sum = 0.0;
    for i in 0..coords.len() {
        let j = (i + 1) % coords.len();
        sum += (coords[j][0] - coords[i][0]) * (coords[j][1] + coords[i][1]);
    }
    sum > 0.0
}

/// Parse apt.dat node lines (111-116) starting from the given line iterator.
/// Returns (parsed rings, number of lines consumed).
/// `mode` = "polygon" for closed shapes (110/130), "line" for open paths (120).
fn parse_apt_path_nodes(lines: &[&str], mode: &str) -> (Vec<ParsedRing>, usize) {
    let mut rings: Vec<ParsedRing> = Vec::new();
    let mut nodes: Vec<AptNode> = Vec::new();
    let mut consumed = 0;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            consumed += 1;
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            consumed += 1;
            continue;
        }

        let row_code: i32 = match parts[0].parse() {
            Ok(v) => v,
            Err(_) => break, // Not a node line, stop consuming
        };

        // Only process node row codes
        if !matches!(row_code, 111 | 112 | 113 | 114 | 115 | 116) {
            break;
        }

        consumed += 1;

        if parts.len() < 3 {
            continue;
        }

        let lat = match parts[1].parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let lon = match parts[2].parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let is_bezier = matches!(row_code, 112 | 114 | 116);
        let closes_ring = matches!(row_code, 113 | 114);
        let ends_path = matches!(row_code, 115 | 116);

        let (ctrl_lat, ctrl_lon, line_type, light_type) = if is_bezier {
            // Bezier: parts[3]=ctrl_lat, parts[4]=ctrl_lon, parts[5]=line_type?, parts[6]=light_type?
            let cl = parts.get(3).and_then(|v| v.parse::<f64>().ok());
            let co = parts.get(4).and_then(|v| v.parse::<f64>().ok());
            let lt = parts
                .get(5)
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0);
            let lgt = parts
                .get(6)
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0);
            (cl, co, lt, lgt)
        } else {
            // Plain node: parts[3]=line_type?, parts[4]=light_type?
            // Disambiguation: single value >= 100 is light_type, < 100 is line_type
            let v3 = parts.get(3).and_then(|v| v.parse::<i32>().ok());
            let v4 = parts.get(4).and_then(|v| v.parse::<i32>().ok());
            let (lt, lgt) = match (v3, v4) {
                (Some(a), Some(b)) => (a, b),
                (Some(a), None) => {
                    if a >= 100 {
                        (0, a)
                    } else {
                        (a, 0)
                    }
                }
                _ => (0, 0),
            };
            (None, None, lt, lgt)
        };

        nodes.push(AptNode {
            lat,
            lon,
            ctrl_lat,
            ctrl_lon,
            line_type,
            light_type,
            closes_ring,
            ends_path,
        });

        // If this node closes a ring, finalize it
        if closes_ring || ends_path {
            if !nodes.is_empty() {
                let ring = resolve_nodes_to_ring(&nodes, mode == "polygon" && closes_ring);
                rings.push(ring);
                nodes.clear();
            }
        }
    }

    // Flush remaining nodes
    if !nodes.is_empty() {
        let ring = resolve_nodes_to_ring(&nodes, false);
        rings.push(ring);
    }

    (rings, consumed)
}

/// Convert a sequence of AptNodes into a ParsedRing with bezier interpolation
fn resolve_nodes_to_ring(nodes: &[AptNode], close: bool) -> ParsedRing {
    let mut coords: Vec<[f64; 2]> = Vec::new();
    let mut node_line_types: Vec<(i32, i32)> = Vec::new();

    if nodes.is_empty() {
        return ParsedRing {
            coords,
            node_line_types,
            is_hole: false,
        };
    }

    // First node always added as-is
    coords.push([nodes[0].lon, nodes[0].lat]);
    node_line_types.push((nodes[0].line_type, nodes[0].light_type));

    for i in 1..nodes.len() {
        let prev = &nodes[i - 1];
        let curr = &nodes[i];

        let prev_has_ctrl = prev.ctrl_lat.is_some() && prev.ctrl_lon.is_some();
        let curr_has_ctrl = curr.ctrl_lat.is_some() && curr.ctrl_lon.is_some();

        let p0 = [prev.lon, prev.lat];
        let p_end = [curr.lon, curr.lat];

        if prev_has_ctrl && curr_has_ctrl {
            // Cubic bezier: prev vertex -> prev outgoing control -> curr mirrored incoming control -> curr vertex
            let ctrl_out = [prev.ctrl_lon.unwrap(), prev.ctrl_lat.unwrap()];
            let ctrl_in_raw = [curr.ctrl_lon.unwrap(), curr.ctrl_lat.unwrap()];
            let ctrl_in = mirror_control(p_end, ctrl_in_raw);
            let pts = bezier_cubic(p0, ctrl_out, ctrl_in, p_end, BEZIER_RESOLUTION);
            for pt in pts {
                coords.push(pt);
                node_line_types.push((curr.line_type, curr.light_type));
            }
        } else if prev_has_ctrl && !curr_has_ctrl {
            // Quadratic bezier: prev outgoing control
            let ctrl_out = [prev.ctrl_lon.unwrap(), prev.ctrl_lat.unwrap()];
            let pts = bezier_quadratic(p0, ctrl_out, p_end, BEZIER_RESOLUTION);
            for pt in pts {
                coords.push(pt);
                node_line_types.push((curr.line_type, curr.light_type));
            }
        } else if !prev_has_ctrl && curr_has_ctrl {
            // Quadratic bezier: curr incoming control (mirrored)
            let ctrl_in_raw = [curr.ctrl_lon.unwrap(), curr.ctrl_lat.unwrap()];
            let ctrl_in = mirror_control(p_end, ctrl_in_raw);
            let pts = bezier_quadratic(p0, ctrl_in, p_end, BEZIER_RESOLUTION);
            for pt in pts {
                coords.push(pt);
                node_line_types.push((curr.line_type, curr.light_type));
            }
        } else {
            // Straight segment
            coords.push(p_end);
            node_line_types.push((curr.line_type, curr.light_type));
        }
    }

    // Close the ring if needed (connect last point back to first, handling bezier)
    if close && coords.len() >= 3 {
        let first = coords[0];
        let last = *coords.last().unwrap();
        if (first[0] - last[0]).abs() > 1e-9 || (first[1] - last[1]).abs() > 1e-9 {
            // Check if last node has bezier that should connect back to first
            let last_node = &nodes[nodes.len() - 1];
            let first_node = &nodes[0];
            if last_node.ctrl_lat.is_some() && first_node.ctrl_lat.is_some() {
                let ctrl_out = [last_node.ctrl_lon.unwrap(), last_node.ctrl_lat.unwrap()];
                let ctrl_in_raw = [first_node.ctrl_lon.unwrap(), first_node.ctrl_lat.unwrap()];
                let ctrl_in = mirror_control(first, ctrl_in_raw);
                let pts = bezier_cubic(last, ctrl_out, ctrl_in, first, BEZIER_RESOLUTION);
                for pt in pts {
                    coords.push(pt);
                    node_line_types.push((first_node.line_type, first_node.light_type));
                }
            } else if last_node.ctrl_lat.is_some() {
                let ctrl_out = [last_node.ctrl_lon.unwrap(), last_node.ctrl_lat.unwrap()];
                let pts = bezier_quadratic(last, ctrl_out, first, BEZIER_RESOLUTION);
                for pt in pts {
                    coords.push(pt);
                    node_line_types.push((first_node.line_type, first_node.light_type));
                }
            } else {
                coords.push(first);
                node_line_types.push((first_node.line_type, first_node.light_type));
            }
        }
    }

    let is_hole = if close { is_clockwise(&coords) } else { false };

    ParsedRing {
        coords,
        node_line_types,
        is_hole,
    }
}

/// Parse a pavement block (row code 110 header + subsequent node lines)
fn parse_pavement_block(
    header_rest: &str,
    following_lines: &[&str],
    airport: &mut AirportDetailBuilder,
) -> usize {
    let parts: Vec<&str> = header_rest.split_whitespace().collect();
    let surface_type = parts
        .first()
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(1);
    let smoothness = parts
        .get(1)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.25);
    let orientation = parts
        .get(2)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);
    let name = if parts.len() > 3 {
        parts[3..].join(" ")
    } else {
        String::new()
    };

    let (rings, consumed) = parse_apt_path_nodes(following_lines, "polygon");
    if rings.is_empty() {
        return consumed;
    }

    // Find outer ring (first non-hole) and holes
    let mut outer_coords: Vec<[f64; 2]> = Vec::new();
    let mut holes: Vec<Vec<[f64; 2]>> = Vec::new();

    for ring in &rings {
        if ring.is_hole {
            holes.push(ring.coords.clone());
        } else if outer_coords.is_empty() {
            outer_coords = ring.coords.clone();
        } else {
            // Additional non-hole rings - treat as separate pavement
            airport.pavements.push(MapPavement {
                surface_type,
                smoothness,
                texture_orientation: orientation,
                name: name.clone(),
                coordinates: ring.coords.clone(),
                holes: Vec::new(),
            });
        }
    }

    if !outer_coords.is_empty() {
        airport.pavements.push(MapPavement {
            surface_type,
            smoothness,
            texture_orientation: orientation,
            name: name.clone(),
            coordinates: outer_coords,
            holes,
        });
    }

    // Extract linear features from node line types (edge markings)
    for ring in &rings {
        extract_linear_features_from_ring(&name, ring, &mut airport.linear_features);
    }

    consumed
}

/// Parse a linear feature block (row code 120 header + subsequent node lines)
fn parse_linear_feature_block(
    header_rest: &str,
    following_lines: &[&str],
    airport: &mut AirportDetailBuilder,
) -> usize {
    let name = header_rest.trim().to_string();

    let (rings, consumed) = parse_apt_path_nodes(following_lines, "line");

    for ring in &rings {
        // Split by line_type changes into separate segments
        split_linear_features(&name, ring, &mut airport.linear_features);
    }

    consumed
}

/// Parse a boundary block (row code 130 header + subsequent node lines)
fn parse_boundary_block(following_lines: &[&str], airport: &mut AirportDetailBuilder) -> usize {
    let (rings, consumed) = parse_apt_path_nodes(following_lines, "polygon");
    if rings.is_empty() {
        return consumed;
    }

    let mut outer_coords: Vec<[f64; 2]> = Vec::new();
    let mut holes: Vec<Vec<[f64; 2]>> = Vec::new();

    for ring in &rings {
        if ring.is_hole {
            holes.push(ring.coords.clone());
        } else if outer_coords.is_empty() {
            outer_coords = ring.coords.clone();
        }
    }

    if !outer_coords.is_empty() {
        airport.boundaries.push(MapBoundary {
            coordinates: outer_coords,
            holes,
        });
    }

    consumed
}

/// Extract linear features from pavement edge markings where nodes have non-zero line types
fn extract_linear_features_from_ring(
    name: &str,
    ring: &ParsedRing,
    features: &mut Vec<MapLinearFeature>,
) {
    if ring.node_line_types.is_empty() || ring.coords.is_empty() {
        return;
    }
    split_linear_features(name, ring, features);
}

/// Split a ring/path into linear feature segments by line_type changes
fn split_linear_features(name: &str, ring: &ParsedRing, features: &mut Vec<MapLinearFeature>) {
    if ring.coords.len() < 2 || ring.node_line_types.is_empty() {
        return;
    }

    let mut current_coords: Vec<[f64; 2]> = vec![ring.coords[0]];
    let mut current_lt = ring.node_line_types[0].0;
    let mut current_lgt = ring.node_line_types[0].1;

    for i in 1..ring.coords.len() {
        let (lt, lgt) = if i < ring.node_line_types.len() {
            ring.node_line_types[i]
        } else {
            (current_lt, current_lgt)
        };

        if lt != current_lt || lgt != current_lgt {
            // Emit previous segment
            if current_lt > 0 || current_lgt > 0 {
                if current_coords.len() >= 2 {
                    features.push(MapLinearFeature {
                        name: name.to_string(),
                        line_type: current_lt,
                        light_type: current_lgt,
                        coordinates: current_coords.clone(),
                    });
                }
            }
            // Start new segment from the last point of the previous segment
            current_coords = vec![ring.coords[i - 1]];
            current_lt = lt;
            current_lgt = lgt;
        }

        current_coords.push(ring.coords[i]);
    }

    // Emit final segment
    if (current_lt > 0 || current_lgt > 0) && current_coords.len() >= 2 {
        features.push(MapLinearFeature {
            name: name.to_string(),
            line_type: current_lt,
            light_type: current_lgt,
            coordinates: current_coords,
        });
    }
}

fn parse_airport_detail_from_apt(
    path: &Path,
    target_icao: &str,
    is_custom: bool,
) -> Result<Option<MapAirportDetail>, String> {
    if !path_exists(path) {
        return Ok(None);
    }

    let file = File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);

    // Collect all lines first so we can do look-ahead for multi-line blocks (110/120/130)
    let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

    let mut current: Option<AirportDetailBuilder> = None;
    let mut i = 0;

    while i < all_lines.len() {
        let trimmed = all_lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        if trimmed == "99" {
            return Ok(current.take().map(|airport| airport.finalize()));
        }

        if is_airport_header(trimmed) {
            if let Some(airport) = current.take() {
                return Ok(Some(airport.finalize()));
            }

            if let Some(header) = parse_airport_header(trimmed, is_custom) {
                if header.icao == target_icao {
                    current = Some(AirportDetailBuilder::from_header(header));
                }
            }
            i += 1;
            continue;
        }

        let Some(airport) = current.as_mut() else {
            i += 1;
            continue;
        };

        if let Some(rest) = trimmed.strip_prefix("100 ") {
            parse_airport_detail_runway(rest, airport);
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("101 ") {
            parse_airport_detail_water_runway(rest, airport);
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("102 ") {
            parse_airport_detail_helipad(rest, airport);
            i += 1;
            continue;
        }

        // Row code 110: Pavement/taxiway area (multi-line block)
        if let Some(rest) = trimmed.strip_prefix("110 ") {
            let following: Vec<&str> = all_lines[i + 1..].iter().map(|s| s.as_str()).collect();
            let consumed = parse_pavement_block(rest, &following, airport);
            i += 1 + consumed;
            continue;
        }

        // Row code 120: Linear feature (multi-line block)
        if let Some(rest) = trimmed.strip_prefix("120 ") {
            let following: Vec<&str> = all_lines[i + 1..].iter().map(|s| s.as_str()).collect();
            let consumed = parse_linear_feature_block(rest, &following, airport);
            i += 1 + consumed;
            continue;
        }

        // Row code 130: Airport boundary (multi-line block)
        if trimmed.starts_with("130 ") || trimmed == "130" {
            let following: Vec<&str> = all_lines[i + 1..].iter().map(|s| s.as_str()).collect();
            let consumed = parse_boundary_block(&following, airport);
            i += 1 + consumed;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("1300 ") {
            parse_airport_detail_start_new(rest, airport);
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("1301 ") {
            parse_airport_detail_start_metadata(rest, airport);
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("15 ") {
            parse_airport_detail_start_legacy(rest, airport);
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("14 ") {
            parse_airport_detail_tower(rest, airport);
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("18 ") {
            parse_airport_detail_beacon(rest, airport);
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("19 ") {
            parse_airport_detail_windsock(rest, airport);
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("20 ") {
            parse_airport_detail_sign(rest, airport);
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("1201 ") {
            parse_airport_detail_taxi_node(rest, airport);
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("1202 ") {
            parse_airport_detail_taxi_edge(rest, airport);
            i += 1;
            continue;
        }

        i += 1;
    }

    Ok(current.take().map(|airport| airport.finalize()))
}

fn load_airport_detail_sync(
    xplane_path: &str,
    icao: &str,
) -> Result<Option<MapAirportDetail>, String> {
    let root = PathBuf::from(xplane_path);
    if !path_exists(&root) {
        return Err(format!("X-Plane path does not exist: {}", xplane_path));
    }

    let target_icao = icao.trim().to_uppercase();
    if target_icao.is_empty() {
        return Ok(None);
    }

    let mut detail: Option<MapAirportDetail> = None;

    let global_path = global_apt_path(&root);
    if let Some(parsed) = parse_airport_detail_from_apt(&global_path, &target_icao, false)? {
        detail = Some(parsed);
    }

    for custom_path in custom_scenery_apt_paths(&root) {
        if let Some(parsed) = parse_airport_detail_from_apt(&custom_path, &target_icao, true)? {
            detail = Some(parsed);
        }
    }

    Ok(detail)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ProcedureCategory {
    Sid,
    Star,
    Approach,
}

impl ProcedureCategory {
    fn as_str(&self) -> &'static str {
        match self {
            ProcedureCategory::Sid => "SID",
            ProcedureCategory::Star => "STAR",
            ProcedureCategory::Approach => "APPROACH",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ProcedureKey {
    category: ProcedureCategory,
    name: String,
    runway: Option<String>,
    transition: Option<String>,
}

fn normalize_upper(input: &str) -> String {
    input.trim().to_uppercase()
}

fn normalize_optional(input: &str) -> Option<String> {
    let value = normalize_upper(input);
    if value.is_empty() || value == "ALL" {
        None
    } else {
        Some(value)
    }
}

fn normalize_runway(input: &str) -> Option<String> {
    normalize_optional(input).map(|value| {
        if value.starts_with("RW") && value.len() > 2 {
            value[2..].to_string()
        } else {
            value
        }
    })
}

fn parse_cifp_line(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let colon = trimmed.find(':')?;
    let row_type = normalize_upper(&trimmed[..colon]);
    if row_type.is_empty() {
        return None;
    }

    let fields = trimmed[colon + 1..]
        .split(',')
        .map(|v| v.trim().to_string())
        .collect::<Vec<_>>();

    if fields.len() < 5 {
        return None;
    }

    Some((row_type, fields))
}

fn classify_procedure(
    row_type: &str,
    route_type: &str,
    field3: &str,
) -> Option<(ProcedureCategory, Option<String>, Option<String>)> {
    match row_type {
        "SID" => {
            let route = route_type.trim();
            let runway = if route == "1" {
                normalize_runway(field3)
            } else {
                None
            };
            let transition = if matches!(route, "4" | "5" | "6") {
                normalize_optional(field3)
            } else {
                None
            };
            Some((ProcedureCategory::Sid, runway, transition))
        }
        "STAR" => {
            let route = route_type.trim();
            let runway = if route == "6" || route == "1" {
                normalize_runway(field3)
            } else {
                None
            };
            let transition = if route == "4" {
                normalize_optional(field3)
            } else {
                None
            };
            Some((ProcedureCategory::Star, runway, transition))
        }
        "APPCH" | "FINAL" => {
            let transition = if route_type.trim() == "A" {
                normalize_optional(field3)
            } else {
                None
            };
            Some((ProcedureCategory::Approach, None, transition))
        }
        _ if row_type.starts_with("RWY") => {
            let runway = normalize_runway(row_type.trim_start_matches("RWY"));
            Some((ProcedureCategory::Approach, runway, None))
        }
        _ => None,
    }
}

fn resolve_cifp_path(xplane_path: &Path, icao: &str) -> Option<PathBuf> {
    let icao_upper = normalize_upper(icao);
    if icao_upper.is_empty() {
        return None;
    }

    let custom = xplane_path
        .join("Custom Data")
        .join("CIFP")
        .join(format!("{}.dat", icao_upper));
    if path_exists(&custom) {
        return Some(custom);
    }

    let default_data = xplane_path
        .join("Resources")
        .join("default data")
        .join("CIFP")
        .join(format!("{}.dat", icao_upper));
    if path_exists(&default_data) {
        return Some(default_data);
    }

    None
}

fn parse_cifp_file(path: &Path, icao: &str) -> Result<MapAirportProcedures, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);
    let target_icao = normalize_upper(icao);

    let mut grouped: HashMap<ProcedureKey, Vec<(i32, MapProcedureWaypoint)>> = HashMap::new();

    for line in reader.lines() {
        let line = match line {
            Ok(v) => v,
            Err(_) => continue,
        };

        let Some((row_type, fields)) = parse_cifp_line(&line) else {
            continue;
        };

        let route_type = fields.get(1).map(|v| v.as_str()).unwrap_or("");
        let name = fields
            .get(2)
            .map(|v| normalize_upper(v))
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }

        let field3 = fields.get(3).map(|v| v.as_str()).unwrap_or("");
        let Some((category, runway, transition)) =
            classify_procedure(&row_type, route_type, field3)
        else {
            continue;
        };

        let fix_id = fields
            .get(4)
            .map(|v| normalize_upper(v))
            .unwrap_or_default();
        if fix_id.is_empty() {
            continue;
        }

        let seq = fields
            .first()
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0);

        let waypoint = MapProcedureWaypoint {
            fix_id,
            fix_region: fields
                .get(5)
                .map(|v| normalize_upper(v))
                .unwrap_or_default(),
            fix_type: fields
                .get(6)
                .map(|v| normalize_upper(v))
                .and_then(|v| v.chars().next().map(|c| c.to_string()))
                .unwrap_or_else(|| "E".to_string()),
            path_terminator: fields
                .get(11)
                .map(|v| normalize_upper(v))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "TF".to_string()),
        };

        let key = ProcedureKey {
            category,
            name,
            runway,
            transition,
        };
        grouped.entry(key).or_default().push((seq, waypoint));
    }

    let mut procedures = MapAirportProcedures {
        icao: target_icao,
        sids: Vec::new(),
        stars: Vec::new(),
        approaches: Vec::new(),
    };

    for (key, mut points) in grouped {
        points.sort_by(|a, b| a.0.cmp(&b.0));
        let waypoints = points.into_iter().map(|(_, wp)| wp).collect::<Vec<_>>();
        if waypoints.is_empty() {
            continue;
        }

        let procedure = MapProcedure {
            procedure_type: key.category.as_str().to_string(),
            name: key.name,
            runway: key.runway,
            transition: key.transition,
            waypoint_count: waypoints.len(),
            waypoints,
        };

        match key.category {
            ProcedureCategory::Sid => procedures.sids.push(procedure),
            ProcedureCategory::Star => procedures.stars.push(procedure),
            ProcedureCategory::Approach => procedures.approaches.push(procedure),
        }
    }

    let sorter = |a: &MapProcedure, b: &MapProcedure| {
        a.name
            .cmp(&b.name)
            .then_with(|| {
                a.runway
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.runway.as_deref().unwrap_or(""))
            })
            .then_with(|| {
                a.transition
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.transition.as_deref().unwrap_or(""))
            })
    };

    procedures.sids.sort_by(sorter);
    procedures.stars.sort_by(sorter);
    procedures.approaches.sort_by(sorter);

    Ok(procedures)
}

fn load_airport_procedures_sync(
    xplane_path: &str,
    icao: &str,
) -> Result<MapAirportProcedures, String> {
    let root = PathBuf::from(xplane_path);
    if !path_exists(&root) {
        return Err(format!("X-Plane path does not exist: {}", xplane_path));
    }

    let target_icao = normalize_upper(icao);
    if target_icao.is_empty() {
        return Err("ICAO is required".to_string());
    }

    let Some(cifp_path) = resolve_cifp_path(&root, &target_icao) else {
        return Ok(MapAirportProcedures {
            icao: target_icao,
            sids: Vec::new(),
            stars: Vec::new(),
            approaches: Vec::new(),
        });
    };

    parse_cifp_file(&cifp_path, &target_icao)
}

fn parse_nav_file(
    path: &Path,
) -> Result<
    (
        Vec<MapNavaid>,
        Vec<MapIls>,
        HashMap<(String, String), (f64, f64)>,
    ),
    String,
> {
    if !path_exists(path) {
        return Ok((Vec::new(), Vec::new(), HashMap::new()));
    }

    let file = File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);

    let mut navaids = Vec::new();
    let mut ils = Vec::new();
    let mut lookup = HashMap::new();

    for (index, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(v) => v,
            Err(_) => continue,
        };
        if index < 2 {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "99" {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }

        let row_code = match parts[0].parse::<i32>() {
            Ok(v) => v,
            Err(_) => continue,
        };

        if !matches!(
            row_code,
            2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 12 | 13 | 14 | 15 | 16
        ) {
            continue;
        }

        let lat = match parts.get(1).and_then(|v| v.parse::<f64>().ok()) {
            Some(v) => v,
            None => continue,
        };
        let lon = match parts.get(2).and_then(|v| v.parse::<f64>().ok()) {
            Some(v) => v,
            None => continue,
        };
        if !is_valid_lat_lon(lat, lon) {
            continue;
        }

        let frequency = parts.get(4).and_then(|v| v.parse::<f64>().ok());
        let id = match parts.get(7) {
            Some(v) => v.to_uppercase(),
            None => continue,
        };
        if id.is_empty() {
            continue;
        }

        let region = match row_code {
            2 | 3 | 12 | 13 => parts.get(8).map(|v| v.to_uppercase()),
            4 | 5 | 6 | 7 | 8 | 9 | 14 | 15 | 16 => parts.get(9).map(|v| v.to_uppercase()),
            _ => None,
        };
        let region_key = region.clone().unwrap_or_default();

        let name = match row_code {
            2 | 3 | 12 | 13 => parts
                .get(10..)
                .map(|s| s.join(" "))
                .unwrap_or_else(String::new),
            4 | 5 | 6 | 7 | 8 | 9 => parts
                .get(11..)
                .map(|s| s.join(" "))
                .unwrap_or_else(String::new),
            14 | 15 => parts
                .get(11..)
                .map(|s| s.join(" "))
                .unwrap_or_else(String::new),
            16 => parts
                .get(12..)
                .map(|s| s.join(" "))
                .unwrap_or_else(String::new),
            _ => String::new(),
        };

        let navaid_type = navaid_type_from_row_code(row_code, &name);
        if navaid_type.is_empty() {
            continue;
        }

        navaids.push(MapNavaid {
            id: id.clone(),
            name: if name.is_empty() {
                id.clone()
            } else {
                name.clone()
            },
            lat,
            lon,
            navaid_type: navaid_type.to_string(),
            frequency,
            region: region.clone(),
        });

        lookup.entry((id.clone(), region_key)).or_insert((lat, lon));

        if let Some(ils_item) = parse_ils_item(row_code, &parts, lat, lon, &id, &name) {
            ils.push(ils_item);
        }
    }

    Ok((navaids, ils, lookup))
}

fn parse_ils_item(
    row_code: i32,
    parts: &[&str],
    lat: f64,
    lon: f64,
    id: &str,
    name: &str,
) -> Option<MapIls> {
    if !matches!(row_code, 4 | 5 | 6 | 14 | 15 | 16) {
        return None;
    }

    let encoded = parts.get(6).and_then(|v| v.parse::<f64>().ok());
    let course = match row_code {
        4 | 5 => encoded.map(|v| v % 360.0),
        6 | 15 | 16 => encoded.map(|v| (v % 100000.0).max(0.0)),
        14 => parts.get(6).and_then(|v| v.parse::<f64>().ok()),
        _ => None,
    };

    let airport = parts.get(8).map(|v| v.to_uppercase());
    let runway = parts.get(10).map(|v| v.to_uppercase());

    Some(MapIls {
        id: id.to_string(),
        name: if name.is_empty() {
            id.to_string()
        } else {
            name.to_string()
        },
        lat,
        lon,
        course,
        airport,
        runway,
    })
}

fn navaid_type_from_row_code(row_code: i32, name: &str) -> &'static str {
    let upper_name = name.to_uppercase();
    match row_code {
        2 => "NDB",
        3 => {
            if upper_name.contains("VORTAC") || upper_name.contains("TACAN") {
                "VORTAC"
            } else if upper_name.contains("VOR/DME") || upper_name.contains("VOR-DME") {
                "VOR-DME"
            } else {
                "VOR"
            }
        }
        4 => "ILS",
        5 => "LOC",
        6 => "GS",
        7 => "OM",
        8 => "MM",
        9 => "IM",
        12 | 13 => {
            if upper_name.contains("TACAN") {
                "TACAN"
            } else {
                "DME"
            }
        }
        14 => "FPAP",
        15 => "GLS",
        16 => "LTP",
        _ => "",
    }
}

fn parse_fix_file(
    path: &Path,
) -> Result<(Vec<MapWaypoint>, HashMap<(String, String), (f64, f64)>), String> {
    if !path_exists(path) {
        return Ok((Vec::new(), HashMap::new()));
    }

    let file = File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);

    let mut waypoints = Vec::new();
    let mut lookup = HashMap::new();

    for (index, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(v) => v,
            Err(_) => continue,
        };
        if index < 2 {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "99" {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let lat = match parts[0].parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let lon = match parts[1].parse::<f64>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        if !is_valid_lat_lon(lat, lon) {
            continue;
        }

        let id = parts[2].to_uppercase();
        if id.is_empty() {
            continue;
        }
        let region = parts[3].to_uppercase();
        let region_opt = Some(region.clone()).filter(|v| !v.is_empty());

        waypoints.push(MapWaypoint {
            id: id.clone(),
            region: region_opt.clone(),
            lat,
            lon,
        });

        lookup.entry((id, region)).or_insert((lat, lon));
    }

    Ok((waypoints, lookup))
}

fn parse_awy_file(
    path: &Path,
    navaids: &HashMap<(String, String), (f64, f64)>,
    waypoints: &HashMap<(String, String), (f64, f64)>,
) -> Result<Vec<MapAirwaySegment>, String> {
    if !path_exists(path) {
        return Ok(Vec::new());
    }

    let file = File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);
    let mut airways = Vec::new();

    let mut id_fallback_lookup: HashMap<String, (f64, f64)> = HashMap::new();
    for ((id, _), coords) in navaids {
        id_fallback_lookup.entry(id.clone()).or_insert(*coords);
    }
    for ((id, _), coords) in waypoints {
        id_fallback_lookup.entry(id.clone()).or_insert(*coords);
    }

    for (index, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(v) => v,
            Err(_) => continue,
        };
        if index < 2 {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "99" {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 11 {
            continue;
        }

        let from_id = parts[0].to_uppercase();
        let from_region = parts[1].to_uppercase();
        let to_id = parts[3].to_uppercase();
        let to_region = parts[4].to_uppercase();

        let from = navaids
            .get(&(from_id.clone(), from_region.clone()))
            .copied()
            .or_else(|| {
                waypoints
                    .get(&(from_id.clone(), from_region.clone()))
                    .copied()
            })
            .or_else(|| id_fallback_lookup.get(&from_id).copied());
        let to = navaids
            .get(&(to_id.clone(), to_region.clone()))
            .copied()
            .or_else(|| waypoints.get(&(to_id.clone(), to_region.clone())).copied())
            .or_else(|| id_fallback_lookup.get(&to_id).copied());

        let (from_lat, from_lon) = match from {
            Some((lat, lon)) => (lat, lon),
            None => continue,
        };
        let (to_lat, to_lon) = match to {
            Some((lat, lon)) => (lat, lon),
            None => continue,
        };

        let is_high = parts[6].eq_ignore_ascii_case("F");
        let base_fl = parts[8].parse::<i32>().ok();
        let top_fl = parts[9].parse::<i32>().ok();
        let name = parts[10].to_uppercase();

        airways.push(MapAirwaySegment {
            name,
            from_id,
            to_id,
            from_lat,
            from_lon,
            to_lat,
            to_lon,
            is_high,
            base_fl,
            top_fl,
        });
    }

    Ok(airways)
}

fn parse_airspace_file(path: &Path) -> Result<Vec<MapAirspace>, String> {
    if !path_exists(path) {
        return Ok(Vec::new());
    }

    let file = File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);

    let mut result = Vec::new();
    let mut class_code = String::new();
    let mut name = String::new();
    let mut upper_limit: Option<String> = None;
    let mut lower_limit: Option<String> = None;
    let mut points: Vec<[f64; 2]> = Vec::new();
    let mut in_block = false;

    let finalize = |result: &mut Vec<MapAirspace>,
                    class_code: &str,
                    name: &str,
                    upper_limit: &Option<String>,
                    lower_limit: &Option<String>,
                    points: &mut Vec<[f64; 2]>| {
        if points.len() < 3 {
            points.clear();
            return;
        }

        if let (Some(first), Some(last)) = (points.first().copied(), points.last().copied()) {
            if (first[0] - last[0]).abs() > f64::EPSILON
                || (first[1] - last[1]).abs() > f64::EPSILON
            {
                points.push(first);
            }
        }

        result.push(MapAirspace {
            name: if name.is_empty() {
                "Unknown".to_string()
            } else {
                name.to_string()
            },
            class_code: if class_code.is_empty() {
                "OTHER".to_string()
            } else {
                class_code.to_string()
            },
            upper_limit: upper_limit.clone(),
            lower_limit: lower_limit.clone(),
            coordinates: points.clone(),
        });
        points.clear();
    };

    for line in reader.lines() {
        let line = match line {
            Ok(v) => v,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('*') {
            continue;
        }

        let cmd = if trimmed.len() >= 2 {
            &trimmed[..2]
        } else {
            ""
        };
        match cmd {
            "AC" => {
                if in_block {
                    finalize(
                        &mut result,
                        &class_code,
                        &name,
                        &upper_limit,
                        &lower_limit,
                        &mut points,
                    );
                }
                in_block = true;
                class_code = normalize_airspace_class(trimmed[2..].trim()).to_string();
                name.clear();
                upper_limit = None;
                lower_limit = None;
            }
            "AN" => {
                if in_block {
                    name = trimmed[2..].trim().to_string();
                }
            }
            "AH" => {
                if in_block {
                    upper_limit = Some(trimmed[2..].trim().to_string());
                }
            }
            "AL" => {
                if in_block {
                    lower_limit = Some(trimmed[2..].trim().to_string());
                }
            }
            "DP" => {
                if in_block {
                    if let Some(point) = parse_airspace_point(trimmed) {
                        points.push(point);
                    }
                }
            }
            _ => {}
        }
    }

    if in_block {
        finalize(
            &mut result,
            &class_code,
            &name,
            &upper_limit,
            &lower_limit,
            &mut points,
        );
    }

    Ok(result)
}

fn normalize_airspace_class(value: &str) -> &'static str {
    match value.trim().to_uppercase().as_str() {
        "A" => "A",
        "B" => "B",
        "C" => "C",
        "D" => "D",
        "E" => "E",
        "F" => "F",
        "G" => "G",
        "CTR" => "CTR",
        "TMA" => "TMA",
        "R" => "R",
        "P" => "P",
        "Q" => "Q",
        "W" => "W",
        "GP" => "GP",
        _ => "OTHER",
    }
}

fn parse_airspace_point(line: &str) -> Option<[f64; 2]> {
    let value = line.trim_start_matches("DP").trim();
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }

    let lat = parse_dms(parts[0], parts[1])?;
    let lon = parse_dms(parts[2], parts[3])?;
    if !is_valid_lat_lon(lat, lon) {
        return None;
    }
    Some([lon, lat])
}

fn parse_dms(dms: &str, direction: &str) -> Option<f64> {
    let pieces: Vec<&str> = dms.split(':').collect();
    if pieces.len() != 3 {
        return None;
    }
    let deg = pieces[0].parse::<f64>().ok()?;
    let min = pieces[1].parse::<f64>().ok()?;
    let sec = pieces[2].parse::<f64>().ok()?;

    let mut value = deg + min / 60.0 + sec / 3600.0;
    let dir = direction.to_ascii_uppercase();
    if dir == "S" || dir == "W" {
        value = -value;
    }
    Some(value)
}

fn is_valid_lat_lon(lat: f64, lon: f64) -> bool {
    lat.is_finite()
        && lon.is_finite()
        && (-90.0..=90.0).contains(&lat)
        && (-180.0..=180.0).contains(&lon)
}

fn surface_code_to_name(code: i32) -> Option<String> {
    let name = match code {
        1 => "asphalt",
        2 => "concrete",
        3 => "turf",
        4 => "dirt",
        5 => "gravel",
        12 => "dry_lakebed",
        13 => "water",
        14 => "snow_ice",
        15 => "transparent",
        _ => return None,
    };
    Some(name.to_string())
}

fn haversine_nm(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r_km = 6371.0_f64;
    let to_rad = |d: f64| d.to_radians();
    let dlat = to_rad(lat2 - lat1);
    let dlon = to_rad(lon2 - lon1);
    let a = (dlat / 2.0).sin().powi(2)
        + to_rad(lat1).cos() * to_rad(lat2).cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    let km = r_km * c;
    km / 1.852
}

fn lon_in_bounds(lon: f64, west: f64, east: f64) -> bool {
    if west <= east {
        (west..=east).contains(&lon)
    } else {
        lon >= west || lon <= east
    }
}

fn ensure_index_loaded_sync(xplane_path: &str) -> Result<(), String> {
    let should_reload = {
        let state = MAP_INDEX_STATE
            .read()
            .map_err(|_| "Map index state read lock poisoned".to_string())?;
        !state.status.loaded || state.status.xplane_path.as_deref() != Some(xplane_path)
    };

    if !should_reload {
        return Ok(());
    }

    let parsed = build_data_index_sync(xplane_path)?;
    let status = MapDataStatus {
        loaded: true,
        xplane_path: Some(xplane_path.to_string()),
        airport_count: parsed.airports.len(),
        navaid_count: parsed.navaids.len(),
        waypoint_count: parsed.waypoints.len(),
        airway_count: parsed.airways.len(),
        ils_count: parsed.ils.len(),
        airspace_count: parsed.airspaces.len(),
        last_loaded_ms: Some(now_unix_ms()),
    };

    let mut state = MAP_INDEX_STATE
        .write()
        .map_err(|_| "Map index state write lock poisoned".to_string())?;
    state.status = status;
    state.airports = parsed.airports;
    state.airport_sources = parsed.airport_sources;
    state.airport_details = HashMap::new();
    state.airport_procedures = HashMap::new();
    state.navaids = parsed.navaids;
    state.waypoints = parsed.waypoints;
    state.airways = parsed.airways;
    state.ils = parsed.ils;
    state.airspaces = parsed.airspaces;

    Ok(())
}

fn get_index_status_snapshot() -> Result<MapDataStatus, String> {
    MAP_INDEX_STATE
        .read()
        .map_err(|_| "Map index state read lock poisoned".to_string())
        .map(|v| v.status.clone())
}

async fn ensure_index_loaded(xplane_path: &str) -> Result<(), String> {
    let should_reload = {
        let state = MAP_INDEX_STATE
            .read()
            .map_err(|_| "Map index state read lock poisoned".to_string())?;
        !state.status.loaded || state.status.xplane_path.as_deref() != Some(xplane_path)
    };

    if !should_reload {
        return Ok(());
    }

    let path = xplane_path.to_string();
    tokio::task::spawn_blocking(move || ensure_index_loaded_sync(&path))
        .await
        .map_err(|e| format!("Map index task join failed: {}", e))?
}

async fn fetch_text(url: &str) -> Result<String, String> {
    let response = HTTP_CLIENT
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed for {}: {}", url, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Request failed for {}: HTTP {}",
            url,
            response.status()
        ));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to decode response for {}: {}", url, e))
}

async fn fetch_json(url: &str) -> Result<Value, String> {
    let response = HTTP_CLIENT
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed for {}: {}", url, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Request failed for {}: HTTP {}",
            url,
            response.status()
        ));
    }

    response
        .json::<Value>()
        .await
        .map_err(|e| format!("Failed to parse JSON for {}: {}", url, e))
}

fn extract_text_payload(input: &str) -> String {
    let mut lines = input.lines();
    let first = lines.next().unwrap_or_default();
    let second = lines.next().unwrap_or_default();

    if !first.is_empty() && second.is_empty() {
        first.trim().to_string()
    } else if !second.is_empty() {
        second.trim().to_string()
    } else {
        input.trim().to_string()
    }
}

async fn resolve_dataref_ids(port: u16) -> Result<HashMap<String, u32>, String> {
    let url = format!("http://localhost:{}/api/v3/datarefs", port);
    let mut req = HTTP_CLIENT.get(url);
    for name in MAP_DATAREFS {
        req = req.query(&[("filter[name]", name)]);
    }
    req = req.query(&[("fields", "id,name"), ("limit", "200")]);

    let response = req
        .send()
        .await
        .map_err(|e| format!("Failed to resolve datarefs: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to resolve datarefs: HTTP {}",
            response.status()
        ));
    }

    let payload = response
        .json::<DatarefCatalogResponse>()
        .await
        .map_err(|e| format!("Failed to parse dataref catalog: {}", e))?;

    let mut out = HashMap::new();
    for item in payload.data.unwrap_or_default() {
        out.insert(item.name, item.id);
    }

    if !out.contains_key(DF_LATITUDE) || !out.contains_key(DF_LONGITUDE) {
        return Err("Could not resolve required latitude/longitude datarefs".to_string());
    }

    Ok(out)
}

async fn fetch_dataref_value(port: u16, dataref_id: u32) -> Result<Option<f64>, String> {
    let url = format!(
        "http://localhost:{}/api/v3/datarefs/{}/value",
        port, dataref_id
    );
    let response = HTTP_CLIENT
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch dataref {}: {}", dataref_id, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch dataref {}: HTTP {}",
            dataref_id,
            response.status()
        ));
    }

    let payload = response
        .json::<DatarefValueResponse>()
        .await
        .map_err(|e| format!("Failed to decode dataref {} value: {}", dataref_id, e))?;

    let Some(data) = payload.data else {
        return Ok(None);
    };

    if let Some(v) = data.as_f64() {
        return Ok(Some(v));
    }

    if let Some(arr) = data.as_array() {
        if let Some(v) = arr.first().and_then(|x| x.as_f64()) {
            return Ok(Some(v));
        }
    }

    Ok(None)
}

async fn fetch_plane_state(
    port: u16,
    ids: &HashMap<String, u32>,
) -> Result<Option<MapPlaneState>, String> {
    let mut values: HashMap<&'static str, f64> = HashMap::new();

    for name in MAP_DATAREFS {
        let Some(id) = ids.get(name) else {
            continue;
        };
        let value = fetch_dataref_value(port, *id).await?;
        if let Some(v) = value {
            values.insert(name, v);
        }
    }

    let lat = match values.get(DF_LATITUDE).copied() {
        Some(v) => v,
        None => return Ok(None),
    };
    let lon = match values.get(DF_LONGITUDE).copied() {
        Some(v) => v,
        None => return Ok(None),
    };

    if !is_valid_lat_lon(lat, lon) {
        return Ok(None);
    }

    let altitude_msl = values.get(DF_ALTITUDE_MSL).map(|v| *v * METERS_TO_FEET);
    let altitude_agl = values.get(DF_ALTITUDE_AGL).map(|v| *v * METERS_TO_FEET);
    let groundspeed = values.get(DF_GROUNDSPEED).map(|v| *v * MPS_TO_KNOTS);

    Ok(Some(MapPlaneState {
        latitude: lat,
        longitude: lon,
        altitude_msl,
        altitude_agl,
        heading: values.get(DF_HEADING).copied(),
        groundspeed,
        indicated_airspeed: values.get(DF_INDICATED_AIRSPEED).copied(),
        vertical_speed: values.get(DF_VERTICAL_SPEED).copied(),
    }))
}

async fn set_plane_connection_state(
    app_handle: &AppHandle,
    controller: &Arc<Mutex<PlaneStreamController>>,
    connected: bool,
) {
    let mut should_emit = false;
    {
        if let Ok(mut state) = controller.lock() {
            if state.connected != connected {
                state.connected = connected;
                should_emit = true;
            }
        }
    }

    if should_emit {
        let _ = app_handle.emit(STREAM_EVENT_CONNECTION, connected);
    }
}

async fn run_plane_stream_task(
    app_handle: AppHandle,
    controller: Arc<Mutex<PlaneStreamController>>,
    mut stop_rx: tokio::sync::watch::Receiver<bool>,
) {
    let port = {
        match controller.lock() {
            Ok(state) => state.port,
            Err(_) => 8086,
        }
    };

    let mut ids: HashMap<String, u32> = HashMap::new();

    loop {
        if *stop_rx.borrow() {
            break;
        }

        if ids.is_empty() {
            match resolve_dataref_ids(port).await {
                Ok(resolved) => {
                    ids = resolved;
                    set_plane_connection_state(&app_handle, &controller, true).await;
                }
                Err(_) => {
                    set_plane_connection_state(&app_handle, &controller, false).await;
                    tokio::select! {
                        _ = stop_rx.changed() => {
                            if *stop_rx.borrow() {
                                break;
                            }
                        }
                        _ = tokio::time::sleep(Duration::from_secs(3)) => {}
                    }
                    continue;
                }
            }
        }

        match fetch_plane_state(port, &ids).await {
            Ok(Some(state)) => {
                set_plane_connection_state(&app_handle, &controller, true).await;
                let _ = app_handle.emit(STREAM_EVENT_STATE, state);
            }
            Ok(None) => {
                set_plane_connection_state(&app_handle, &controller, false).await;
            }
            Err(_) => {
                ids.clear();
                set_plane_connection_state(&app_handle, &controller, false).await;
            }
        }

        tokio::select! {
            _ = stop_rx.changed() => {
                if *stop_rx.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(850)) => {}
        }
    }

    set_plane_connection_state(&app_handle, &controller, false).await;
}

#[tauri::command]
pub async fn map_prepare_data_index(xplane_path: String) -> Result<MapDataStatus, String> {
    ensure_index_loaded(&xplane_path).await?;

    get_index_status_snapshot()
}

#[tauri::command]
pub fn map_get_data_status() -> Result<MapDataStatus, String> {
    get_index_status_snapshot()
}

#[tauri::command]
pub async fn map_search_airports(
    xplane_path: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<MapAirport>, String> {
    ensure_index_loaded(&xplane_path).await?;

    let q = query.trim().to_uppercase();
    if q.is_empty() {
        return Ok(Vec::new());
    }

    let limit = limit.unwrap_or(20).clamp(1, 100);

    let state = MAP_INDEX_STATE
        .read()
        .map_err(|_| "Map index state read lock poisoned".to_string())?;

    let q_lower = q.to_lowercase();
    let mut rows: Vec<MapAirport> = state
        .airports
        .iter()
        .filter(|airport| {
            airport.icao.contains(&q) || airport.name.to_lowercase().contains(&q_lower)
        })
        .cloned()
        .collect();

    rows.sort_by(|a, b| airport_search_cmp(a, b, &q));
    rows.truncate(limit);
    Ok(rows)
}

fn airport_search_cmp(a: &MapAirport, b: &MapAirport, q: &str) -> Ordering {
    let score = |airport: &MapAirport| -> (i32, usize, usize) {
        let icao = airport.icao.as_str();
        let name = airport.name.to_uppercase();
        let p1 = if icao == q {
            0
        } else if icao.starts_with(q) {
            1
        } else if icao.contains(q) {
            2
        } else if name.starts_with(q) {
            3
        } else {
            4
        };
        (p1, icao.len(), name.len())
    };

    score(a).cmp(&score(b)).then_with(|| a.icao.cmp(&b.icao))
}

#[tauri::command]
pub async fn map_get_airports_in_bounds(
    xplane_path: String,
    bounds: MapBounds,
    limit: Option<usize>,
) -> Result<Vec<MapAirport>, String> {
    ensure_index_loaded(&xplane_path).await?;

    let limit = limit.unwrap_or(1200).clamp(1, 10_000);

    let state = MAP_INDEX_STATE
        .read()
        .map_err(|_| "Map index state read lock poisoned".to_string())?;

    let mut out: Vec<MapAirport> = state
        .airports
        .iter()
        .filter(|airport| {
            airport.lat >= bounds.south
                && airport.lat <= bounds.north
                && lon_in_bounds(airport.lon, bounds.west, bounds.east)
        })
        .cloned()
        .collect();

    out.sort_by(|a, b| a.icao.cmp(&b.icao));
    out.truncate(limit);
    Ok(out)
}

#[tauri::command]
pub async fn map_get_airport_detail(
    xplane_path: String,
    icao: String,
) -> Result<MapAirportDetail, String> {
    ensure_index_loaded(&xplane_path).await?;

    let icao = icao.trim().to_uppercase();
    if icao.is_empty() {
        return Err("ICAO is required".to_string());
    }

    let source_hint = {
        let state = MAP_INDEX_STATE
            .read()
            .map_err(|_| "Map index state read lock poisoned".to_string())?;
        if let Some(detail) = state.airport_details.get(&icao) {
            return Ok(detail.clone());
        }
        state.airport_sources.get(&icao).cloned()
    };

    let xplane_path_clone = xplane_path.clone();
    let icao_clone = icao.clone();
    let detail = tokio::task::spawn_blocking(move || {
        if let Some(source) = source_hint {
            let path = PathBuf::from(&source.apt_path);
            if let Some(parsed) =
                parse_airport_detail_from_apt(&path, &icao_clone, source.is_custom)?
            {
                return Ok(Some(parsed));
            }
        }
        load_airport_detail_sync(&xplane_path_clone, &icao_clone)
    })
    .await
    .map_err(|e| format!("Airport detail task join failed: {}", e))??;

    let Some(detail) = detail else {
        return Err(format!("Airport detail not found for {}", icao));
    };

    let mut state = MAP_INDEX_STATE
        .write()
        .map_err(|_| "Map index state write lock poisoned".to_string())?;
    state.airport_details.insert(icao, detail.clone());

    Ok(detail)
}

#[tauri::command]
pub async fn map_get_airport_procedures(
    xplane_path: String,
    icao: String,
) -> Result<MapAirportProcedures, String> {
    ensure_index_loaded(&xplane_path).await?;

    let icao = normalize_upper(&icao);
    if icao.is_empty() {
        return Err("ICAO is required".to_string());
    }

    {
        let state = MAP_INDEX_STATE
            .read()
            .map_err(|_| "Map index state read lock poisoned".to_string())?;
        if let Some(cached) = state.airport_procedures.get(&icao) {
            return Ok(cached.clone());
        }
    }

    let xplane_path_clone = xplane_path.clone();
    let icao_clone = icao.clone();
    let procedures = tokio::task::spawn_blocking(move || {
        load_airport_procedures_sync(&xplane_path_clone, &icao_clone)
    })
    .await
    .map_err(|e| format!("Airport procedures task join failed: {}", e))??;

    let mut state = MAP_INDEX_STATE
        .write()
        .map_err(|_| "Map index state write lock poisoned".to_string())?;
    state.airport_procedures.insert(icao, procedures.clone());

    Ok(procedures)
}

#[tauri::command]
pub async fn map_get_nav_snapshot(
    xplane_path: String,
    request: MapLayerRequest,
) -> Result<MapNavSnapshot, String> {
    ensure_index_loaded(&xplane_path).await?;

    let radius_nm = request.radius_nm.clamp(5.0, 400.0);
    let include_navaids = request.include_navaids.unwrap_or(true);
    let include_waypoints = request.include_waypoints.unwrap_or(true);
    let include_airways = request.include_airways.unwrap_or(true);
    let include_ils = request.include_ils.unwrap_or(true);
    let include_airspaces = request.include_airspaces.unwrap_or(true);

    let state = MAP_INDEX_STATE
        .read()
        .map_err(|_| "Map index state read lock poisoned".to_string())?;

    let mut snapshot = MapNavSnapshot::default();

    if include_navaids {
        snapshot.navaids = state
            .navaids
            .iter()
            .filter(|item| haversine_nm(request.lat, request.lon, item.lat, item.lon) <= radius_nm)
            .take(3500)
            .cloned()
            .collect();
    }

    if include_waypoints {
        snapshot.waypoints = state
            .waypoints
            .iter()
            .filter(|item| haversine_nm(request.lat, request.lon, item.lat, item.lon) <= radius_nm)
            .take(4500)
            .cloned()
            .collect();
    }

    if include_ils {
        snapshot.ils = state
            .ils
            .iter()
            .filter(|item| haversine_nm(request.lat, request.lon, item.lat, item.lon) <= radius_nm)
            .take(1500)
            .cloned()
            .collect();
    }

    if include_airways {
        snapshot.airways = state
            .airways
            .iter()
            .filter(|item| {
                let d1 = haversine_nm(request.lat, request.lon, item.from_lat, item.from_lon);
                let d2 = haversine_nm(request.lat, request.lon, item.to_lat, item.to_lon);
                d1 <= radius_nm || d2 <= radius_nm
            })
            .take(5000)
            .cloned()
            .collect();
    }

    if include_airspaces {
        let lat_delta = radius_nm / 60.0;
        let cos_lat = request.lat.to_radians().cos().abs().max(0.1);
        let lon_delta = lat_delta / cos_lat;
        let min_lat = request.lat - lat_delta;
        let max_lat = request.lat + lat_delta;
        let min_lon = request.lon - lon_delta;
        let max_lon = request.lon + lon_delta;

        snapshot.airspaces = state
            .airspaces
            .iter()
            .filter(|airspace| {
                airspace.coordinates.iter().any(|coord| {
                    let lon = coord[0];
                    let lat = coord[1];
                    lat >= min_lat && lat <= max_lat && lon >= min_lon && lon <= max_lon
                })
            })
            .take(600)
            .cloned()
            .collect();
    }

    Ok(snapshot)
}

#[tauri::command]
pub async fn map_fetch_metar(icao: String) -> Result<String, String> {
    let icao = icao.trim().to_uppercase();
    if icao.is_empty() {
        return Ok(String::new());
    }

    let url = format!("https://metar.vatsim.net/{}", icao);
    let text = fetch_text(&url).await?;
    Ok(extract_text_payload(&text))
}

#[tauri::command]
pub async fn map_fetch_taf(icao: String) -> Result<String, String> {
    let icao = icao.trim().to_uppercase();
    if icao.is_empty() {
        return Ok(String::new());
    }

    let url = format!(
        "https://aviationweather.gov/api/data/taf?ids={}&format=raw",
        icao
    );
    let text = fetch_text(&url).await?;
    Ok(extract_text_payload(&text))
}

#[tauri::command]
pub async fn map_fetch_vatsim_data() -> Result<Value, String> {
    fetch_json("https://data.vatsim.net/v3/vatsim-data.json").await
}

#[tauri::command]
pub async fn map_fetch_vatsim_events() -> Result<Value, String> {
    fetch_json("https://my.vatsim.net/api/v2/events/latest").await
}

#[tauri::command]
pub async fn map_fetch_vatsim_metar(icao: String) -> Result<String, String> {
    let icao = icao.trim().to_uppercase();
    if icao.is_empty() {
        return Ok(String::new());
    }

    let url = format!("https://metar.vatsim.net/{}", icao);
    let text = fetch_text(&url).await?;
    Ok(extract_text_payload(&text))
}

#[tauri::command]
pub async fn map_fetch_rainviewer_manifest() -> Result<Value, String> {
    fetch_json("https://api.rainviewer.com/public/weather-maps.json").await
}

#[tauri::command]
pub async fn map_fetch_simbrief_latest(pilot_id: String) -> Result<Value, String> {
    let pilot_id = pilot_id.trim();
    if pilot_id.is_empty() {
        return Ok(serde_json::json!({
            "success": false,
            "error": "Pilot ID is required"
        }));
    }

    let url = format!(
        "https://www.simbrief.com/api/xml.fetcher.php?userid={}&json=1",
        pilot_id
    );
    fetch_json(&url).await
}

#[tauri::command]
pub async fn map_fetch_gateway_airport(icao: String) -> Result<Value, String> {
    let icao = icao.trim().to_uppercase();
    if icao.is_empty() {
        return Err("ICAO is required".to_string());
    }
    let url = format!("https://gateway.x-plane.com/apiv1/airport/{}", icao);
    fetch_json(&url).await
}

#[tauri::command]
pub async fn map_fetch_gateway_scenery(scenery_id: i64) -> Result<Value, String> {
    if scenery_id <= 0 {
        return Err("Scenery ID must be a positive integer".to_string());
    }
    let url = format!("https://gateway.x-plane.com/apiv1/scenery/{}", scenery_id);
    fetch_json(&url).await
}

#[tauri::command]
pub async fn map_start_plane_stream(
    app_handle: AppHandle,
    port: Option<u16>,
) -> Result<bool, String> {
    let stream_port = port.unwrap_or(8086);
    let controller = PLANE_STREAM_STATE.clone();
    let mut guard = controller
        .lock()
        .map_err(|_| "Plane stream state lock poisoned".to_string())?;

    if guard.running {
        return Ok(true);
    }

    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    guard.running = true;
    guard.connected = false;
    guard.port = stream_port;
    guard.stop_tx = Some(stop_tx);
    drop(guard);

    let task_controller = controller.clone();
    tauri::async_runtime::spawn(async move {
        run_plane_stream_task(app_handle, task_controller.clone(), stop_rx).await;
        if let Ok(mut state) = task_controller.lock() {
            state.running = false;
            state.connected = false;
            state.stop_tx = None;
        }
    });

    Ok(true)
}

#[tauri::command]
pub fn map_stop_plane_stream() -> Result<bool, String> {
    let controller = PLANE_STREAM_STATE.clone();
    let mut guard = controller
        .lock()
        .map_err(|_| "Plane stream state lock poisoned".to_string())?;

    if !guard.running {
        return Ok(true);
    }

    if let Some(tx) = guard.stop_tx.take() {
        let _ = tx.send(true);
    }

    guard.running = false;
    guard.connected = false;
    Ok(true)
}

#[tauri::command]
pub fn map_get_plane_stream_status() -> Result<MapPlaneStreamStatus, String> {
    let guard = PLANE_STREAM_STATE
        .lock()
        .map_err(|_| "Plane stream state lock poisoned".to_string())?;
    Ok(MapPlaneStreamStatus {
        running: guard.running,
        connected: guard.connected,
        port: guard.port,
    })
}

// ---------------------------------------------------------------------------
// X-Plane REST API: dataref read/write and command execution
// ---------------------------------------------------------------------------

/// Check if the X-Plane REST API is reachable.
#[tauri::command]
pub async fn xplane_is_api_available(port: Option<u16>) -> Result<bool, String> {
    let p = port.unwrap_or(8086);
    let url = format!("http://localhost:{}/api/v3/datarefs?limit=1", p);
    match HTTP_CLIENT
        .get(&url)
        .timeout(Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}

/// Resolve a single dataref name to its numeric ID.
async fn resolve_single_dataref_id(port: u16, name: &str) -> Result<u32, String> {
    let url = format!("http://localhost:{}/api/v3/datarefs", port);
    let response = HTTP_CLIENT
        .get(&url)
        .query(&[
            ("filter[name]", name),
            ("fields", "id,name"),
            ("limit", "1"),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to resolve dataref '{}': {}", name, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Dataref resolve HTTP {}: {}",
            response.status(),
            name
        ));
    }

    let payload = response
        .json::<DatarefCatalogResponse>()
        .await
        .map_err(|e| format!("Failed to parse dataref resolve response: {}", e))?;

    payload
        .data
        .and_then(|v| v.into_iter().next())
        .map(|entry| entry.id)
        .ok_or_else(|| format!("Dataref not found: {}", name))
}

/// Resolve a single command name to its numeric ID.
async fn resolve_command_id(port: u16, name: &str) -> Result<u32, String> {
    let url = format!("http://localhost:{}/api/v3/commands", port);
    let response = HTTP_CLIENT
        .get(&url)
        .query(&[
            ("filter[name]", name),
            ("fields", "id,name"),
            ("limit", "1"),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to resolve command '{}': {}", name, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Command resolve HTTP {}: {}",
            response.status(),
            name
        ));
    }

    let payload = response
        .json::<DatarefCatalogResponse>()
        .await
        .map_err(|e| format!("Failed to parse command resolve response: {}", e))?;

    payload
        .data
        .and_then(|v| v.into_iter().next())
        .map(|entry| entry.id)
        .ok_or_else(|| format!("Command not found: {}", name))
}

/// Get a dataref value by name. Returns JSON value (number, array, or string).
#[tauri::command]
pub async fn xplane_get_dataref(
    port: Option<u16>,
    name: String,
    index: Option<u32>,
) -> Result<Value, String> {
    let p = port.unwrap_or(8086);
    let id = resolve_single_dataref_id(p, &name).await?;

    let mut url = format!("http://localhost:{}/api/v3/datarefs/{}/value", p, id);
    if let Some(idx) = index {
        url = format!("{}?index={}", url, idx);
    }

    let response = HTTP_CLIENT
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to get dataref '{}': {}", name, e))?;

    if !response.status().is_success() {
        return Err(format!("Get dataref '{}' HTTP {}", name, response.status()));
    }

    let payload = response
        .json::<DatarefValueResponse>()
        .await
        .map_err(|e| format!("Failed to decode dataref '{}': {}", name, e))?;

    Ok(payload.data.unwrap_or(Value::Null))
}

/// Set a dataref value by name.
#[tauri::command]
pub async fn xplane_set_dataref(
    port: Option<u16>,
    name: String,
    value: Value,
    index: Option<u32>,
) -> Result<bool, String> {
    let p = port.unwrap_or(8086);
    let id = resolve_single_dataref_id(p, &name).await?;

    let mut url = format!("http://localhost:{}/api/v3/datarefs/{}/value", p, id);
    if let Some(idx) = index {
        url = format!("{}?index={}", url, idx);
    }

    let body = serde_json::json!({ "data": value });
    let response = HTTP_CLIENT
        .patch(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to set dataref '{}': {}", name, e))?;

    if !response.status().is_success() {
        return Err(format!("Set dataref '{}' HTTP {}", name, response.status()));
    }

    Ok(true)
}

/// Execute an X-Plane command by name.
#[tauri::command]
pub async fn xplane_activate_command(
    port: Option<u16>,
    name: String,
    duration: Option<f64>,
) -> Result<bool, String> {
    let p = port.unwrap_or(8086);
    let id = resolve_command_id(p, &name).await?;

    let url = format!("http://localhost:{}/api/v3/commands/{}/activate", p, id);
    let body = serde_json::json!({ "duration": duration.unwrap_or(0.0) });

    let response = HTTP_CLIENT
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to activate command '{}': {}", name, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Activate command '{}' HTTP {}",
            name,
            response.status()
        ));
    }

    Ok(true)
}

// ---------------------------------------------------------------------------
// Aircraft scanning & flight launch
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedLivery {
    pub name: String,
    pub folder: String,
    pub preview_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedAircraft {
    pub path: String,
    pub name: String,
    pub icao: String,
    pub description: String,
    pub manufacturer: String,
    pub studio: String,
    pub author: String,
    pub tail_number: String,
    pub empty_weight_lbs: f64,
    pub max_weight_lbs: f64,
    pub max_fuel_lbs: f64,
    pub tank_count: usize,
    pub tank_names: Vec<String>,
    pub tank_ratios: Vec<f64>,
    pub payload_stations: Vec<PayloadStation>,
    pub is_helicopter: bool,
    pub engine_count: u32,
    pub preview_image: Option<String>,
    pub liveries: Vec<ScannedLivery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadStation {
    pub name: String,
    pub max_weight_lbs: f64,
}

fn parse_acf_file(acf_path: &Path, xplane_root: &Path) -> Option<ScannedAircraft> {
    let file = File::open(acf_path).ok()?;
    let reader = BufReader::new(file);
    let mut props: HashMap<String, String> = HashMap::new();

    for line in reader.lines() {
        let line = match line {
            Ok(v) => v,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if !trimmed.starts_with("P acf/") {
            continue;
        }
        // P acf/_name My Aircraft Name
        if let Some(rest) = trimmed.strip_prefix("P ") {
            let mut parts = rest.splitn(2, char::is_whitespace);
            if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                props.insert(key.to_string(), val.trim().to_string());
            }
        }
    }

    let name = props.get("acf/_name").cloned().unwrap_or_default();
    if name.is_empty() {
        return None;
    }

    let rel_path = acf_path
        .strip_prefix(xplane_root)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");

    // Parse tank names/ratios (up to 9)
    let mut tank_names = Vec::new();
    let mut tank_ratios = Vec::new();
    for i in 0..9 {
        let tn = props
            .get(&format!("acf/_tank_name/{}", i))
            .cloned()
            .unwrap_or_default();
        let tr = props
            .get(&format!("acf/_tank_rat/{}", i))
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        if tr > 0.0 || !tn.is_empty() {
            tank_names.push(if tn.is_empty() {
                format!("Tank {}", i + 1)
            } else {
                tn
            });
            tank_ratios.push(tr);
        }
    }

    // Parse payload stations (up to 9)
    let mut payload_stations = Vec::new();
    for i in 0..9 {
        let pn = props
            .get(&format!("acf/_fixed_name/{}", i))
            .cloned()
            .unwrap_or_default();
        let pm = props
            .get(&format!("acf/_fixed_max/{}", i))
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        if pm > 0.0 || !pn.is_empty() {
            payload_stations.push(PayloadStation {
                name: if pn.is_empty() {
                    format!("Station {}", i + 1)
                } else {
                    pn
                },
                max_weight_lbs: pm,
            });
        }
    }

    // Scan liveries
    let acf_dir = acf_path.parent()?;
    let liveries_dir = acf_dir.join("liveries");
    let mut liveries = vec![ScannedLivery {
        name: "Default".to_string(),
        folder: String::new(),
        preview_path: None,
    }];

    if liveries_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&liveries_dir) {
            let mut dirs: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                .collect();
            dirs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
            for entry in dirs {
                let folder = entry.file_name().to_string_lossy().to_string();
                if folder.starts_with('.') {
                    continue;
                }
                // Look for preview image
                let preview = find_preview_in_dir(&entry.path());
                liveries.push(ScannedLivery {
                    name: folder.clone(),
                    folder,
                    preview_path: preview,
                });
            }
        }
    }

    // Find preview for default livery
    let default_preview = find_preview_in_dir(acf_dir);
    if let Some(p) = default_preview {
        liveries[0].preview_path = Some(p);
    }

    Some(ScannedAircraft {
        path: rel_path,
        name,
        icao: props.get("acf/_ICAO").cloned().unwrap_or_default(),
        description: props.get("acf/_descrip").cloned().unwrap_or_default(),
        manufacturer: props.get("acf/_manufacturer").cloned().unwrap_or_default(),
        studio: props.get("acf/_studio").cloned().unwrap_or_default(),
        author: props.get("acf/_author").cloned().unwrap_or_default(),
        tail_number: props.get("acf/_tailnum").cloned().unwrap_or_default(),
        empty_weight_lbs: props
            .get("acf/_m_empty")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0),
        max_weight_lbs: props
            .get("acf/_m_max")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0),
        max_fuel_lbs: props
            .get("acf/_m_fuel_max_tot")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0),
        tank_count: tank_ratios.len(),
        tank_names,
        tank_ratios,
        payload_stations,
        is_helicopter: props
            .get("acf/_is_helicopter")
            .map(|v| v == "1")
            .unwrap_or(false),
        engine_count: props
            .get("acf/_num_engn")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        preview_image: liveries.first().and_then(|l| l.preview_path.clone()),
        liveries,
    })
}

fn find_preview_in_dir(dir: &Path) -> Option<String> {
    // Look for _icon11.png first, then any _icon*.png
    let candidates = ["_icon11.png", "_icon.png"];
    for name in &candidates {
        let path = dir.join(name);
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }
    // Fallback: find any PNG that contains "icon"
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let fname = entry.file_name().to_string_lossy().to_lowercase();
            if fname.contains("icon") && fname.ends_with(".png") && !fname.contains("thumb") {
                return Some(entry.path().to_string_lossy().to_string());
            }
        }
    }
    None
}

fn scan_aircraft_sync(xplane_path: &str) -> Result<Vec<ScannedAircraft>, String> {
    let root = PathBuf::from(xplane_path);
    let aircraft_dir = root.join("Aircraft");
    if !aircraft_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    fn walk_dir(dir: &Path, xplane_root: &Path, results: &mut Vec<ScannedAircraft>, depth: u32) {
        if depth > 5 {
            return;
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str.to_lowercase() == "liveries" {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                walk_dir(&path, xplane_root, results, depth + 1);
            } else if name_str.to_lowercase().ends_with(".acf") {
                if let Some(aircraft) = parse_acf_file(&path, xplane_root) {
                    results.push(aircraft);
                }
            }
        }
    }

    walk_dir(&aircraft_dir, &root, &mut results, 0);
    results.sort_by(|a, b| {
        a.manufacturer
            .cmp(&b.manufacturer)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(results)
}

#[tauri::command]
pub async fn map_scan_aircraft(xplane_path: String) -> Result<Vec<ScannedAircraft>, String> {
    tokio::task::spawn_blocking(move || scan_aircraft_sync(&xplane_path))
        .await
        .map_err(|e| format!("Scan aircraft task failed: {}", e))?
}

#[tauri::command]
pub async fn map_get_aircraft_image(image_path: String) -> Result<String, String> {
    let path = PathBuf::from(&image_path);
    if !path.exists() {
        return Err("Image file not found".to_string());
    }

    let data = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("Failed to read image: {}", e))?;

    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
    Ok(format!("data:image/png;base64,{}", encoded))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LaunchFlightRequest {
    pub xplane_path: String,
    pub aircraft_path: String,
    pub livery_folder: Option<String>,
    pub airport_icao: String,
    pub start_position: Option<String>,
    pub start_is_runway: bool,
    pub fuel_weights_kg: Vec<f64>,
    pub payload_weights_kg: Vec<f64>,
    pub time_hours: Option<f64>,
    pub day_of_year: Option<u32>,
    pub weather_preset: Option<String>,
}

#[tauri::command]
pub async fn map_launch_flight(request: LaunchFlightRequest) -> Result<bool, String> {
    let root = PathBuf::from(&request.xplane_path);
    if !root.is_dir() {
        return Err("X-Plane path not found".to_string());
    }

    // Build start position JSON
    let start_json = if request.start_is_runway {
        format!(
            r#"{{"runway_start":{{"airport_id":"{}","runway":"{}"}}}}"#,
            request.airport_icao,
            request.start_position.as_deref().unwrap_or("")
        )
    } else {
        format!(
            r#"{{"ramp_start":{{"airport_id":"{}","ramp":"{}"}}}}"#,
            request.airport_icao,
            request.start_position.as_deref().unwrap_or("")
        )
    };

    // Build fuel array (always 9 slots)
    let mut fuel_slots = vec![0.0f64; 9];
    for (i, &w) in request.fuel_weights_kg.iter().enumerate() {
        if i < 9 {
            fuel_slots[i] = w;
        }
    }

    // Build payload array
    let payload_slots: Vec<f64> = request.payload_weights_kg.clone();

    // Build the flight JSON for --new_flight_json
    let flight_json = serde_json::json!({
        "aircraft": request.aircraft_path,
        "livery": request.livery_folder.as_deref().unwrap_or(""),
        "departure_airport": request.airport_icao,
        "start": serde_json::from_str::<Value>(&start_json).unwrap_or(Value::Null),
        "weight": {
            "fueltank_weight_in_kilograms": fuel_slots,
            "payload_weight_in_kilograms": payload_slots,
        },
    });

    // Write to temp file
    let temp_dir = std::env::temp_dir();
    let json_path = temp_dir.join("xfast-manager-flight.json");
    let json_str = serde_json::to_string_pretty(&flight_json)
        .map_err(|e| format!("Failed to serialize flight JSON: {}", e))?;

    tokio::fs::write(&json_path, json_str)
        .await
        .map_err(|e| format!("Failed to write flight JSON: {}", e))?;

    // Find X-Plane executable
    let exe_path = if cfg!(target_os = "windows") {
        root.join("X-Plane.exe")
    } else if cfg!(target_os = "macos") {
        root.join("X-Plane.app/Contents/MacOS/X-Plane")
    } else {
        root.join("X-Plane-x86_64")
    };

    if !exe_path.exists() {
        return Err(format!(
            "X-Plane executable not found at: {}",
            exe_path.display()
        ));
    }

    // Launch X-Plane with --new_flight_json flag
    std::process::Command::new(&exe_path)
        .arg(format!("--new_flight_json={}", json_path.display()))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to launch X-Plane: {}", e))?;

    Ok(true)
}
