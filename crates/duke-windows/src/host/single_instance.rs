use crate::host::win32::wide_null_terminated;
use std::ptr::null;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::Threading::CreateMutexW;

pub const WINDOW_CLASS_OF_A_RUNNING_INSTANCE: &str = "DukePixelArtBackground";
const MUTEX_HELD_BY_THE_RUNNING_INSTANCE: &str = r"Local\DukePixelArtBackground.SingleInstance";

pub fn claim_for_the_life_of_this_process() -> bool {
    let mutex_name = wide_null_terminated(MUTEX_HELD_BY_THE_RUNNING_INSTANCE);
    let mutex_kept_open_until_the_process_exits = unsafe { CreateMutexW(null(), 0, mutex_name.as_ptr()) };
    if mutex_kept_open_until_the_process_exits.is_null() {
        return false;
    }
    let another_instance_created_it_first = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if another_instance_created_it_first {
        unsafe { CloseHandle(mutex_kept_open_until_the_process_exits) };
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_claim_is_refused_while_the_first_is_held() {
        let _first_claim_may_already_belong_to_a_real_instance = claim_for_the_life_of_this_process();
        assert!(!claim_for_the_life_of_this_process());
        assert!(!claim_for_the_life_of_this_process());
    }
}
