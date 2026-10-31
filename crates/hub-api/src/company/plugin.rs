//! The company's link to the plugin in the player's openOMSI (the old `omsihub/addon.rs`, what
//! the company's pages called of it): the duty a drive is to drive, given to the plugin with the
//! company's parts of it; the plugin's side of the control room's messages; the notices for the
//! driver.
//!
//! The company builds all of it (the duty sheet, the fare, the contract's note, the crew's
//! strip: `launch::plugin_parts`, `launch::crew_of`); [`crate::addon`] writes it for the plugin
//! when the drive starts (`drive::start` → `addon::give`), and books what the plugin reports.

use hub_game::bridge;
use std::path::PathBuf;

pub use crate::addon::Assigned;

/// The company's parts of a duty's assignment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parts {
    pub sheet: Vec<bridge::SheetRow>,
    pub controls: Vec<bridge::ControlNote>,
    pub fare: Option<bridge::Fare>,
    pub contract: Option<bridge::ContractNote>,
    /// The bus taken over halfway along its trip (`bridge::TakeOver`).
    pub take_over: Option<bridge::TakeOver>,
}

/// Give the plugin the duty of a drive about to start: `company` names the company's duty (its
/// name and the duty's id) and gives its parts, `crew` the driver's strip and the bus's notes,
/// `company_id` the company its notes, defects and live events go to. Kept until the drive
/// `d` starts (`addon::give` writes it then, with the rest of the assignment); a drive of the
/// player's own gets an assignment of its own there.
pub fn assign_duty(d: &hub_core::Duty, company: Option<(&str, &str, Parts)>, crew: Option<bridge::Crew>, company_id: Option<&str>) {
    let part = company.map(|(name, duty, parts)| crate::addon::CompanyPart {
        name: name.to_string(),
        duty: duty.to_string(),
        parts,
        crew,
        company_id: company_id.unwrap_or_default().to_string(),
        bus: d.bus.clone(),
        map: d.map.clone(),
    });
    crate::addon::set_company(part);
}

/// The plugin's folder and the company's duty it was given, while it has one (the messages'
/// link).
pub fn duty_link() -> Option<(PathBuf, Assigned)> {
    crate::addon::duty_link()
}

/// Cards for the driver in the game Omsi-Hub started (`data/notices.json`).
pub fn notify_game(notices: Vec<bridge::Notice>) {
    crate::addon::notify_game(notices)
}
