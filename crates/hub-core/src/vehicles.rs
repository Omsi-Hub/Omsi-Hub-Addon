//! The buses installed: what the company's market, the dealer and the fleet show of each.
//! Read with `hub-omsi`'s `.bus` reader through the content folders ([`crate::vfs`]): the
//! `Vehicles` folder's `.bus` files, the content folder's first.

use serde::{Deserialize, Serialize};

/// An installed bus, as the company's pages list it (the old launcher's JSON field names).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct VehicleInfo {
    /// The name it is shown with (manufacturer and type).
    pub name: String,
    pub manufacturer: String,
    pub type_name: String,
    /// Its `.bus` file, relative to the OMSI 2 folder (`Vehicles/MAN_SD202/MAN_SD202.bus`).
    pub file: String,
    /// Its folder under `Vehicles`.
    pub folder: String,
    pub description: String,
    /// The livery its `[friendlyname]` names as its own.
    pub default_paint: String,
    /// Its liveries (`.cti` items), where known.
    pub paints: Vec<String>,
    /// The depot files it comes with.
    pub hofs: Vec<String>,
    /// It is in openOMSI's content folder (a mod), not in the OMSI 2 folder.
    pub installed: bool,
    /// Vehicle packs it borrows parts from that are not installed.
    pub missing_packs: Vec<String>,
    /// The fleet numbers of its `[number]` list with the plate each comes with.
    pub numbers: Vec<(String, String)>,
}

impl VehicleInfo {
    /// The bus `bus` read from `file` (relative to the OMSI 2 folder) in `folder`.
    pub fn of_bus(bus: &hub_omsi::vehicle::Bus, file: &str, folder: &str, installed: bool, hofs: Vec<String>) -> VehicleInfo {
        let type_name = crate::vehicle_type_label(&bus.type_name, std::path::Path::new(file));
        let name = if bus.manufacturer.trim().is_empty() { type_name.clone() } else { format!("{} {}", bus.manufacturer.trim(), type_name) };
        VehicleInfo {
            name,
            manufacturer: bus.manufacturer.trim().to_string(),
            type_name,
            file: file.to_string(),
            folder: folder.to_string(),
            description: bus.description.clone(),
            default_paint: bus.default_paint.clone(),
            paints: Vec::new(),
            hofs,
            installed,
            missing_packs: Vec::new(),
            numbers: bus.numbers.clone(),
        }
    }
}

/// Every bus installed (not the AI's only, not a rear section), sorted by name. Empty when the
/// OMSI 2 folder is not set.
pub fn list_vehicles() -> anyhow::Result<Vec<VehicleInfo>> {
    let Some(vfs) = crate::paths::vfs() else { return Ok(Vec::new()) };
    Ok(vehicles_in(&vfs))
}

/// The buses of the content folders `vfs`.
pub fn vehicles_in(vfs: &hub_omsi::Vfs) -> Vec<VehicleInfo> {
    let mut out: Vec<VehicleInfo> = Vec::new();
    for (folder, is_dir) in vfs.list_dir("Vehicles") {
        if !is_dir {
            continue;
        }
        let rel_dir = format!("Vehicles/{folder}");
        let files = vfs.list_dir(&rel_dir);
        let hofs: Vec<String> = files.iter().filter(|(n, d)| !d && n.to_lowercase().ends_with(".hof")).map(|(n, _)| n.clone()).collect();
        for (name, d) in &files {
            if *d || !name.to_lowercase().ends_with(".bus") {
                continue;
            }
            let rel = format!("{rel_dir}/{name}");
            let Some(path) = vfs.resolve(&rel) else { continue };
            let Ok(bus) = hub_omsi::vehicle::Bus::load(&path) else { continue };
            if bus.is_traffic() || bus.is_rear_section() {
                continue;
            }
            let installed = vfs.bases().len() > 1 && path.starts_with(&vfs.bases()[0]);
            let mut v = VehicleInfo::of_bus(&bus, &rel, &folder, installed, hofs.clone());
            // (its paints in every content folder, as openOMSI offers them: the company's sheets
            // choose from them)
            v.paints = bus.paints_in_vfs(vfs).into_iter().filter(|p| !p.trim().is_empty()).collect();
            out.push(v);
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// A bus file as the company reads it: `hub-omsi`'s header ([`hub_omsi::vehicle::Bus`]), and
/// what the market's guesses look at beyond it - the file names of its scripts, constant files
/// and variable lists (`[script]`, `[constfile]`, `[varnamelist]`), and how far left or right
/// its first driver's camera sits (`[add_camera_driver]`, the side the wheel is on).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BusFile {
    pub bus: hub_omsi::vehicle::Bus,
    pub parts: Vec<String>,
    pub driver_x: Option<f32>,
}

impl BusFile {
    pub fn parse(cfg: &hub_omsi::CfgFile) -> BusFile {
        let bus = hub_omsi::vehicle::Bus::parse(cfg);
        let mut parts: Vec<String> = Vec::new();
        for kw in ["script", "constfile", "varnamelist"] {
            for block in cfg.blocks(kw) {
                let n = block.first().map(|l| hub_omsi::text::parse_i32(l).max(0) as usize).unwrap_or(0);
                for l in block.iter().skip(1).take(n) {
                    let name = l.trim().rsplit(['\\', '/']).next().unwrap_or("").to_string();
                    if !name.is_empty() {
                        parts.push(name);
                    }
                }
            }
        }
        let driver_x = cfg.blocks("add_camera_driver").first().and_then(|b| b.first()).map(|l| hub_omsi::text::parse_f32(l));
        BusFile { bus, parts, driver_x }
    }

    pub fn load(path: &std::path::Path) -> Result<BusFile, hub_omsi::ReadError> {
        let mut b = BusFile::parse(&hub_omsi::CfgFile::read(path)?);
        b.bus.path = path.to_path_buf();
        Ok(b)
    }

    /// The section coupled behind, its file found beside this one.
    pub fn couple_back_path(&self) -> Option<std::path::PathBuf> {
        let (file, _) = self.bus.couple_back.as_ref()?;
        hub_omsi::vfs::resolve_in(self.bus.dir(), file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bus_is_named_by_its_maker_and_type() {
        let bus = hub_omsi::vehicle::Bus { manufacturer: "MAN".into(), type_name: "SD_202".into(), numbers: vec![("1".into(), "B-1".into())], ..Default::default() };
        let v = VehicleInfo::of_bus(&bus, "Vehicles/SD202/a.bus", "SD202", false, vec![]);
        assert_eq!((v.name.as_str(), v.type_name.as_str()), ("MAN SD 202", "SD 202"));
        let back: VehicleInfo = serde_json::from_value(serde_json::to_value(&v).unwrap()).unwrap();
        assert_eq!(back, v);
        // (an older JSON without the newer fields reads)
        let old: VehicleInfo = serde_json::from_str(r#"{"name":"X","file":"a.bus"}"#).unwrap();
        assert!(old.numbers.is_empty());
    }

    #[test]
    #[ignore = "needs hub-omsi port"]
    fn a_bus_file_s_parts_and_driver_s_seat() {
        let text = "[friendlyname]\nMAN\nSD202\nweiss\n\n[script]\n2\nscript\\main.osc\nscript/elec.osc\n\n[varnamelist]\n1\nscript\\varlist.txt\n\n[add_camera_driver]\n0.55\n1.6\n";
        let b = BusFile::parse(&hub_omsi::CfgFile::from_str("a.bus", text));
        assert_eq!(b.parts, ["main.osc", "elec.osc", "varlist.txt"]);
        assert_eq!(b.driver_x, Some(0.55));
    }
}
