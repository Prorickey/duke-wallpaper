use crate::host::win32::wide_null_terminated;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
    REG_SZ, RRF_RT_REG_SZ,
};

const PROGRAMS_RUN_AT_LOGON_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const OUR_VALUE_NAME: &str = "DukePixelArtBackground";
const BYTES_PER_WIDE_CHARACTER: usize = 2;

pub fn is_enabled() -> bool {
    let key_path = wide_null_terminated(PROGRAMS_RUN_AT_LOGON_KEY);
    let value_name = wide_null_terminated(OUR_VALUE_NAME);
    let asking_only_whether_it_exists = null_mut();
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key_path.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            asking_only_whether_it_exists,
            null_mut(),
        )
    };
    result == ERROR_SUCCESS
}

pub fn set_enabled_because_the_user_clicked_the_menu_item(enabled: bool) -> Result<(), String> {
    let key_path = wide_null_terminated(PROGRAMS_RUN_AT_LOGON_KEY);
    let value_name = wide_null_terminated(OUR_VALUE_NAME);
    let quoted_path_of_this_program = if enabled {
        let this_program =
            std::env::current_exe().map_err(|error| format!("could not find this executable: {error}"))?;
        Some(wide_null_terminated(&format!("\"{}\"", this_program.display())))
    } else {
        None
    };

    let mut run_key: HKEY = null_mut();
    let open_result = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, key_path.as_ptr(), 0, KEY_SET_VALUE, &mut run_key) };
    if open_result != ERROR_SUCCESS {
        return Err(format!("could not open the Run key (error {open_result})"));
    }
    let change_result = match &quoted_path_of_this_program {
        Some(command) => unsafe {
            let command_bytes = (command.len() * BYTES_PER_WIDE_CHARACTER) as u32;
            RegSetValueExW(run_key, value_name.as_ptr(), 0, REG_SZ, command.as_ptr().cast(), command_bytes)
        },
        None => unsafe { RegDeleteValueW(run_key, value_name.as_ptr()) },
    };
    unsafe { RegCloseKey(run_key) };

    let was_already_absent = !enabled && change_result == ERROR_FILE_NOT_FOUND;
    if change_result != ERROR_SUCCESS && !was_already_absent {
        return Err(format!("registry update failed (error {change_result})"));
    }
    Ok(())
}
