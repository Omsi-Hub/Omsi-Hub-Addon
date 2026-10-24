//! The buses' depot models (the 3D depot page): the light model the page bakes of a bus in a
//! livery with its own renderer (`ui/js/hub/bus3d/bake.js` - an atlas of five flat views and a
//! hull of a few hundred triangles), kept beside the photos in `~/.openomsi/cache/busphotos/`
//! as `d1-<id>.png` and `d1-<id>.json` (the hull, and the files it was made from).
//!
//! A model goes as a photo goes: when one of the files of the bus or of its livery changed
//! (`photo::stamps_of`, checked at most once a minute); the page then bakes it again.
//!
//! * `b3d_depot_has { bus, paint }`: `{ has, model }`, the hull when it is kept and fresh;
//! * `b3d_depot_put { bus, paint, png, model }` (the PNG in base64): keep what the page baked;
//! * bytes: `b3d_depot { bus, paint }`, the atlas's PNG.

use super::photo;
use super::tex;
use crate::Answer;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

/// The way the models are made: as the page's `BAKE_STYLE`; raised with it.
const STYLE: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Kept {
    bus: String,
    paint: String,
    stamps: Vec<(PathBuf, u64, u64)>,
    model: Value,
}

fn arg_str(args: &Value, name: &str) -> String {
    args.get(name).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// The model's name for a bus file and a livery, in the looks as they are worked out now (as
/// a photo's).
pub fn name_of(bus: &str, paint: &str) -> String {
    format!("d{STYLE}-{}", tex::id_of(&format!("depot-{STYLE}|looks-{}|{}|{}", super::cache::LOOKS, bus.replace('\\', "/").trim().to_lowercase(), paint.trim().to_lowercase())))
}

/// The model of `bus` in `paint` and its atlas, if they are kept for the bus as it is now.
fn kept(bus: &str, paint: &str) -> Option<(Kept, PathBuf)> {
    let name = name_of(bus, paint);
    let png = photo::dir().join(format!("{name}.png"));
    let k: Kept = serde_json::from_slice(&std::fs::read(photo::dir().join(format!("{name}.json"))).ok()?).ok()?;
    (png.is_file() && photo::fresh(&name, &k.stamps)).then_some((k, png))
}

/// `b3d_depot_has { bus, paint }`.
pub fn has(args: &Value) -> Answer {
    Ok(match kept(&arg_str(args, "bus"), &arg_str(args, "paint")) {
        Some((k, _)) => json!({ "has": true, "model": k.model }),
        None => json!({ "has": false }),
    })
}

/// `b3d_depot { bus, paint }`: the kept atlas's PNG.
pub fn get(args: &Value) -> Result<Vec<u8>, String> {
    let (_, png) = kept(&arg_str(args, "bus"), &arg_str(args, "paint")).ok_or_else(|| "no depot model".to_string())?;
    std::fs::read(&png).map_err(|e| e.to_string())
}

/// `b3d_depot_put { bus, paint, png, model }`.
pub fn put(args: &Value) -> Answer {
    let (bus, paint) = (arg_str(args, "bus"), arg_str(args, "paint"));
    let png = photo::base64(&arg_str(args, "png"))?;
    if !png.starts_with(b"\x89PNG") {
        return Err("not a PNG".into());
    }
    let model = args.get("model").cloned().unwrap_or(Value::Null);
    if model.get("verts").and_then(Value::as_array).is_none_or(|v| v.is_empty() || v.len() % 8 != 0) {
        return Err("no hull".into());
    }
    let root = super::roots()?;
    let stamps = photo::stamps_of(&root, &bus, &paint);
    if stamps.is_empty() {
        return Err("the bus is not open".into());
    }
    let name = name_of(&bus, &paint);
    let d = photo::dir();
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    std::fs::write(d.join(format!("{name}.png")), &png).map_err(|e| e.to_string())?;
    let kept = Kept { bus, paint, stamps, model };
    std::fs::write(d.join(format!("{name}.json")), serde_json::to_vec(&kept).unwrap_or_default()).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_depot_model_is_named_after_the_bus_and_its_livery_apart_from_its_photo() {
        assert_eq!(name_of("Vehicles\\MAN_SD200\\SD200.bus", "BVG"), name_of("vehicles/man_sd200/sd200.bus", " bvg "));
        assert_ne!(name_of("Vehicles/MAN_SD200/SD200.bus", "BVG"), name_of("Vehicles/MAN_SD200/SD200.bus", ""));
        assert_ne!(name_of("a", "b"), photo::name_of("a", "b"));
        assert!(name_of("a", "b").starts_with("d1-"));
    }

    #[test]
    fn a_model_without_its_hull_is_refused() {
        let png = "iVBORw0KGgo=";
        assert!(put(&json!({ "bus": "a", "paint": "", "png": png, "model": { "verts": [] } })).is_err());
        assert!(put(&json!({ "bus": "a", "paint": "", "png": png, "model": { "verts": [1, 2, 3] } })).is_err());
        assert!(put(&json!({ "bus": "a", "paint": "", "png": "aGVsbG8=", "model": { "verts": [0, 0, 0, 0, 0, 0, 0, 0] } })).is_err());
    }
}
