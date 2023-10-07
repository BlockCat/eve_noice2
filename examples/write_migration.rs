use chrono::{Datelike, Timelike};
use futures::TryStreamExt;
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::File,
    path::Path,
};

struct SDE {
    market_groups: HashMap<usize, EveMarketGroup>,
    groups: HashMap<usize, EveGroup>,
    types: HashMap<TypeId, EveType>,
    regions: HashMap<RegionId, String>,
    // systems: HashMap<SystemId, (String, usize, Vec<usize>)>,
    systems: HashMap<SystemId, SdeSystem>,
}

fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(start());
}

async fn start() {
    let sqlx = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect("database.db")
        .await
        .unwrap();

    let timestamp = chrono::Utc::now();
    let migration_path = Path::new("./migrations").join(format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}_update_sde.sql",
        timestamp.year(),
        timestamp.month(),
        timestamp.day(),
        timestamp.hour(),
        timestamp.minute(),
        timestamp.second()
    ));

    println!("Writing migration to: {:?}", migration_path);

    let mut migration = String::new();
    let sde = read_sde("./data/sde");

    migration.push_str(&create_region_migrations(&sqlx, &sde).await);
    migration.push('\n');

    migration.push_str(&create_system_migrations(&sqlx, &sde).await);
    migration.push('\n');

    migration.push_str(&create_stargates_migrations(&sde).await);
    migration.push('\n');

    migration.push_str(&create_group_migrations(&sqlx, &sde).await);
    migration.push('\n');

    migration.push_str(&create_market_group_migrations(&sqlx, &sde).await);
    migration.push('\n');

    migration.push_str(&create_type_migrations(&sqlx, &sde).await);
    migration.push('\n');

    std::fs::write(migration_path, migration).unwrap();
}

async fn create_region_migrations(pool: &Pool<Sqlite>, sde: &SDE) -> String {
    let mut migration = String::new();
    let existing_regions = sqlx::query!("SELECT id FROM eve_region")
        .map(|s| RegionId(s.id as usize))
        .fetch(pool)
        .try_collect::<HashSet<_>>()
        .await
        .unwrap();

    let mut deleted_regions = existing_regions
        .difference(&sde.regions.keys().copied().collect::<HashSet<_>>())
        .copied()
        .collect::<Vec<_>>();

    deleted_regions.sort();

    if !deleted_regions.is_empty() {
        migration.push_str(&format!(
            "DELETE FROM eve_region WHERE id IN ({});\n",
            deleted_regions
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }

    for region in sde.regions.iter() {
        let region_id = *region.0;
        let region_name = region.1;

        if !existing_regions.contains(&region_id) {
            migration.push_str(
                &format!(
                    "INSERT OR REPLACE INTO eve_region (id, name) VALUES ({}, '{}');\n",
                    region_id,
                    clean_name(region_name)
                )
                .to_string(),
            );
        }
    }

    migration
}

async fn create_stargates_migrations(sde: &SDE) -> String {
    let mut migration = String::new();

    let stargate_system = sde
        .systems
        .iter()
        .flat_map(|(system_id, sde_system)| {
            sde_system.stargates.iter().map(move |b| (b.0, *system_id))
        })
        .collect::<HashMap<_, _>>();

    for system in sde.systems.iter() {
        let source_system_id = *system.0;

        let system_static_data = &system.1.stargates;

        for dest_stargate in system_static_data {
            migration.push_str(
                &format!(
                    "INSERT OR REPLACE INTO eve_stargates (source_system_id, target_system_id) VALUES ({}, {});\n",
                    source_system_id, stargate_system[&dest_stargate.1]
                )
                .to_string(),
            );
        }
    }

    migration
}

async fn create_system_migrations(pool: &Pool<Sqlite>, sde: &SDE) -> String {
    let mut migration = String::new();
    let existing_systems = sqlx::query!("SELECT id FROM eve_system")
        .map(|s| SystemId(s.id as usize))
        .fetch(pool)
        .try_collect::<HashSet<_>>()
        .await
        .unwrap();

    let mut deleted_systems = existing_systems
        .difference(&sde.systems.keys().copied().collect::<HashSet<_>>())
        .copied()
        .collect::<Vec<_>>();

    deleted_systems.sort();

    if !deleted_systems.is_empty() {
        migration.push_str(&format!(
            "DELETE FROM eve_system WHERE id IN ({});\n",
            deleted_systems
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }

    for system in sde.systems.iter() {
        let system_id = *system.0;
        if !existing_systems.contains(&system_id) {
            migration.push_str(&system.1.to_sql());
        }
    }

    migration
}

async fn create_group_migrations(pool: &Pool<Sqlite>, sde: &SDE) -> String {
    let mut migration = String::new();
    let existing_groups = sqlx::query!("SELECT id FROM eve_groups")
        .map(|s| s.id as usize)
        .fetch(pool)
        .try_collect::<HashSet<_>>()
        .await
        .unwrap();

    let mut deleted_groups = existing_groups
        .difference(&sde.groups.keys().copied().collect::<HashSet<_>>())
        .copied()
        .collect::<Vec<_>>();

    deleted_groups.sort();

    if !deleted_groups.is_empty() {
        migration.push_str(&format!(
            "DELETE FROM eve_groups WHERE id IN ({});\n",
            deleted_groups
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }

    for group in sde.groups.iter() {
        let group_id = *group.0;
        let group_name = &group.1.name.en;

        if !existing_groups.contains(&group_id) {
            migration.push_str(
                &format!(
                    "INSERT OR REPLACE INTO eve_groups (id, name) VALUES ({}, '{}');\n",
                    group_id,
                    clean_name(group_name)
                )
                .to_string(),
            );
        }
    }

    migration
}

async fn create_market_group_migrations(pool: &Pool<Sqlite>, sde: &SDE) -> String {
    let mut migration = String::new();
    let existing_market_groups = sqlx::query!("SELECT id FROM eve_market_groups")
        .map(|s| s.id as usize)
        .fetch(pool)
        .try_collect::<HashSet<_>>()
        .await
        .unwrap();

    let mut deleted_market_groups = existing_market_groups
        .difference(&sde.market_groups.keys().copied().collect::<HashSet<_>>())
        .copied()
        .collect::<Vec<_>>();

    deleted_market_groups.sort();

    if !deleted_market_groups.is_empty() {
        migration.push_str(&format!(
            "DELETE FROM eve_market_groups WHERE id IN ({});\n",
            deleted_market_groups
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }

    for market_group in sde.market_groups.iter() {
        let market_group_id = *market_group.0;
        let market_group_name = &market_group.1.name.en;

        if !existing_market_groups.contains(&market_group_id) {
            migration.push_str(
                &format!(
                    "INSERT OR REPLACE INTO eve_market_groups (id, name) VALUES ({}, '{}');\n",
                    market_group_id,
                    clean_name(market_group_name)
                )
                .to_string(),
            );
        }
    }

    migration
}

async fn create_type_migrations(pool: &Pool<Sqlite>, sde: &SDE) -> String {
    let mut migration = String::new();
    let existing_types = sqlx::query!("SELECT id FROM eve_items")
        .map(|s| TypeId(s.id as usize))
        .fetch(pool)
        .try_collect::<HashSet<_>>()
        .await
        .unwrap();

    let mut deleted_types = existing_types
        .difference(&sde.types.keys().copied().collect::<HashSet<_>>())
        .copied()
        .collect::<Vec<_>>();

    deleted_types.sort();

    if !deleted_types.is_empty() {
        migration.push_str(&format!(
            "DELETE FROM eve_items WHERE id IN ({});\n",
            deleted_types
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }

    for type_ in sde.types.iter() {
        let type_id = *type_.0;
        let type_name = &type_.1.name.en;
        let group_id = type_.1.group_id;
        let market_group_id = type_.1.market_group_id;

        if !sde.groups.contains_key(&group_id) {
            panic!("Missing group ID");
        }

        if let Some(market_group_id) = market_group_id {
            if !sde.market_groups.contains_key(&market_group_id) {
                panic!("Missing market group ID: {}", market_group_id);
            }
        }

        if !existing_types.contains(&type_id) {
            migration.push_str(
            &format!(
                "INSERT OR REPLACE INTO eve_items (id, name, group_id, market_group_id, published) VALUES ({}, '{}', {}, {}, {});\n",
                type_id, clean_name(type_name), group_id, market_group_id.map(|x| x.to_string()).unwrap_or("NULL".to_string()), type_.1.published as i32
            )
            .to_string(),
        );
        }
    }
    migration
}

fn read_sde(path: &str) -> SDE {
    println!("Reading SDE from: {}", path);
    let path = Path::new(path);

    println!("Loading market groups");
    let market_groups = load_market_groups(path);

    println!("Inserting root market group");

    println!("Loading groups");
    let groups = load_groups(path);

    println!("Loading types");
    let types = load_types(path);

    println!("Loading regions");
    let (regions, systems) = load_regions(path);

    SDE {
        market_groups,
        groups,
        types,
        regions,
        systems,
    }
}

fn load_types(path: &Path) -> HashMap<TypeId, EveType> {
    serde_yaml::from_reader::<_, HashMap<TypeId, EveType>>(
        File::open(path.join("fsd/typeIDs.yaml")).unwrap(),
    )
    .unwrap()
    .into_iter()
    .collect::<HashMap<_, _>>()
}

fn load_groups(path: &Path) -> HashMap<usize, EveGroup> {
    // let group_ids = load_file(&path.join("fsd/groupIDs.yaml").to_str().unwrap());
    serde_yaml::from_reader::<_, HashMap<usize, EveGroup>>(
        File::open(path.join("fsd/groupIDs.yaml")).unwrap(),
    )
    .unwrap()
    .into_iter()
    .collect()
}

fn load_market_groups(path: &Path) -> HashMap<usize, EveMarketGroup> {
    serde_yaml::from_reader::<_, HashMap<usize, EveMarketGroup>>(
        File::open(path.join("fsd/marketGroups.yaml")).unwrap(),
    )
    .unwrap()
}

fn load_regions(path: &Path) -> (HashMap<RegionId, String>, HashMap<SystemId, SdeSystem>) {
    let folder = path.join("fsd/universe/eve");
    let regions = std::fs::read_dir(folder).expect("Could not read folder: fsd/universe/eve");

    let inv_names = serde_yaml::from_reader::<_, Vec<InvItem>>(
        File::open(path.join("bsd/invNames.yaml")).unwrap(),
    )
    .unwrap()
    .into_iter()
    .map(|x| (x.item_id, x.item_name))
    .collect::<HashMap<_, _>>();

    let mut region_map = HashMap::new();
    let mut system_map = HashMap::new();

    for region in regions {
        let region = region.unwrap();

        let region_static_data = serde_yaml::from_reader::<_, RegionStaticData>(
            File::open(region.path().join("region.staticdata")).unwrap(),
        )
        .unwrap();

        region_map.insert(
            region_static_data.region_id,
            inv_names[&region_static_data.region_id.0].clone(),
        );
        let constellations = std::fs::read_dir(&region.path()).unwrap();
        for constellation in constellations {
            let constellation = constellation.unwrap();
            if !constellation.path().is_dir() {
                continue;
            }
            let solar_systems = std::fs::read_dir(&constellation.path()).unwrap();
            for system in solar_systems {
                let system = system.unwrap();
                let static_data = system.path().join("solarsystem.staticdata");
                if system.path().is_dir() {
                    let static_data: SolarSystemStaticData = serde_yaml::from_reader(
                        File::open(system.path().join("solarsystem.staticdata"))
                            .unwrap_or_else(|_| panic!("Could not read file: {:?}", static_data)),
                    )
                    .unwrap();

                    system_map.insert(
                        static_data.solar_system_id,
                        SdeSystem {
                            id: static_data.solar_system_id,
                            name: inv_names[&static_data.solar_system_id.0].clone(),
                            region_id: region_static_data.region_id,
                            stargates: static_data
                                .stargates
                                .iter()
                                .map(|(gate_id, static_data)| {
                                    (*gate_id, static_data.destination_gate)
                                })
                                .collect(),
                        },
                    );
                }
            }
        }
    }

    (region_map, system_map)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvItem {
    #[serde(rename = "itemID")]
    item_id: usize,
    item_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EveType {
    name: Translation,
    #[serde(rename = "groupID")]
    group_id: usize,
    published: bool,
    #[serde(rename = "marketGroupID")]
    market_group_id: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EveGroup {
    name: Translation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EveMarketGroup {
    #[serde(rename = "nameID")]
    name: Translation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Translation {
    en: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegionStaticData {
    #[serde(rename = "regionID")]
    region_id: RegionId,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SolarSystemStaticData {
    #[serde(rename = "solarSystemID")]
    solar_system_id: SystemId,
    stargates: HashMap<usize, SolarSystemStaticGateData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SolarSystemStaticGateData {
    #[serde(rename = "destination")]
    destination_gate: usize,
}

fn clean_name(name: &str) -> String {
    name.trim().replace('\'', "''").replace('\"', "\"\"")
}

#[derive(Debug, sqlx::FromRow, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
struct SystemId(usize);

impl Display for SystemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

#[derive(Debug, sqlx::FromRow, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
struct RegionId(usize);

impl Display for RegionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

#[derive(Debug, sqlx::FromRow, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
struct TypeId(usize);

impl Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

struct SdeSystem {
    id: SystemId,
    name: String,
    region_id: RegionId,
    stargates: Vec<(usize, usize)>, // source stargate id, target stargate id
}

impl SdeSystem {
    fn new(
        id: SystemId,
        name: String,
        region_id: RegionId,
        stargates: Vec<(usize, usize)>,
    ) -> Self {
        Self {
            id,
            name,
            region_id,
            stargates,
        }
    }

    fn to_sql(&self) -> String {
        format!(
            "INSERT OR REPLACE INTO eve_system (id, name, region_id) VALUES ({}, '{}', {});",
            self.id,
            clean_name(&self.name),
            self.region_id
        )
    }
}
