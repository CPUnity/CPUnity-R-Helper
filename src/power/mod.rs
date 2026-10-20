use anyhow::Result;

#[cfg(target_os = "windows")]
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

#[cfg(target_os = "windows")]
pub fn get_power_state() -> Result<bool> {
    unsafe {
        let mut status: SYSTEM_POWER_STATUS = std::mem::zeroed();
        if GetSystemPowerStatus(&mut status).is_ok() {
            // 255 means "Windows does not know yet": an error, never "on battery"
            crate::power_plan::ac_flag(status.ACLineStatus)
                .ok_or_else(|| anyhow::anyhow!("charger state unknown"))
        } else {
            Err(anyhow::anyhow!("GetSystemPowerStatus failed"))
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_power_state() -> Result<bool> {
    Ok(true)
}
