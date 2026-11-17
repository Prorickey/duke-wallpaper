#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PowerEvent {
    SessionLocked,
    SessionUnlocked,
    DisplayTurnedOff,
    DisplayTurnedOn,
    SystemSuspending,
    SystemResumed,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct PauseReasons {
    pub paused_by_user: bool,
    pub session_is_locked: bool,
    pub display_is_off: bool,
    pub system_is_suspending: bool,
    pub running_on_battery: bool,
}

impl PauseReasons {
    pub fn note(&mut self, event: PowerEvent) {
        match event {
            PowerEvent::SessionLocked => self.session_is_locked = true,
            PowerEvent::SessionUnlocked => {
                self.session_is_locked = false;
                self.the_display_is_evidently_on();
            }
            PowerEvent::DisplayTurnedOff => self.display_is_off = true,
            PowerEvent::DisplayTurnedOn => self.the_display_is_evidently_on(),
            PowerEvent::SystemSuspending => self.system_is_suspending = true,
            PowerEvent::SystemResumed => self.the_system_is_evidently_awake(),
        }
    }

    fn the_display_is_evidently_on(&mut self) {
        self.display_is_off = false;
        self.the_system_is_evidently_awake();
    }

    fn the_system_is_evidently_awake(&mut self) {
        self.system_is_suspending = false;
    }

    pub fn reason_not_to_render(&self, pause_on_battery: bool) -> Option<&'static str> {
        if self.paused_by_user {
            return Some("paused");
        }
        if self.system_is_suspending {
            return Some("system suspending");
        }
        if self.session_is_locked {
            return Some("session locked");
        }
        if self.display_is_off {
            return Some("display off");
        }
        if pause_on_battery && self.running_on_battery {
            return Some("on battery");
        }
        None
    }

    pub fn system_can_be_asked_whether_the_reason_still_holds(&self, pause_on_battery: bool) -> bool {
        self.session_is_locked || (pause_on_battery && self.running_on_battery)
    }
}

#[cfg(test)]
mod tests {
    use super::PowerEvent::*;
    use super::*;

    fn every_order_of(events: &[PowerEvent]) -> Vec<Vec<PowerEvent>> {
        if events.len() <= 1 {
            return vec![events.to_vec()];
        }
        let mut orders = Vec::new();
        for first_position in 0..events.len() {
            let mut remaining = events.to_vec();
            let first = remaining.remove(first_position);
            for mut order_of_the_rest in every_order_of(&remaining) {
                order_of_the_rest.insert(0, first);
                orders.push(order_of_the_rest);
            }
        }
        orders
    }

    #[test]
    fn rendering_resumes_whatever_order_the_events_arrive_in() {
        for going_away in every_order_of(&[SessionLocked, DisplayTurnedOff, SystemSuspending]) {
            for coming_back in every_order_of(&[SessionUnlocked, DisplayTurnedOn, SystemResumed]) {
                let mut pause_reasons = PauseReasons::default();
                for &event in &going_away {
                    pause_reasons.note(event);
                    assert!(pause_reasons.reason_not_to_render(false).is_some());
                }
                for &event in &coming_back {
                    pause_reasons.note(event);
                }
                assert_eq!(pause_reasons.reason_not_to_render(false), None, "{going_away:?} then {coming_back:?}");
            }
        }
    }

    #[test]
    fn started_while_locked_with_the_display_off() {
        for coming_back in every_order_of(&[SessionUnlocked, DisplayTurnedOn]) {
            let mut pause_reasons =
                PauseReasons { session_is_locked: true, display_is_off: true, ..PauseReasons::default() };
            assert_eq!(pause_reasons.reason_not_to_render(false), Some("session locked"));
            pause_reasons.note(coming_back[0]);
            let still_locked_behind_a_lit_display = coming_back[0] == DisplayTurnedOn;
            let expected = if still_locked_behind_a_lit_display { Some("session locked") } else { None };
            assert_eq!(pause_reasons.reason_not_to_render(false), expected);
            pause_reasons.note(coming_back[1]);
            assert_eq!(pause_reasons.reason_not_to_render(false), None);
        }
    }

    #[test]
    fn one_lost_notification_cannot_freeze_the_wallpaper_for_good() {
        let mut resume_never_announced = PauseReasons::default();
        resume_never_announced.note(SystemSuspending);
        resume_never_announced.note(DisplayTurnedOff);
        resume_never_announced.note(DisplayTurnedOn);
        assert_eq!(resume_never_announced.reason_not_to_render(false), None);

        let mut display_on_never_announced = PauseReasons::default();
        display_on_never_announced.note(DisplayTurnedOff);
        display_on_never_announced.note(SessionLocked);
        display_on_never_announced.note(SessionUnlocked);
        assert_eq!(display_on_never_announced.reason_not_to_render(false), None);

        let mut unlock_never_announced = PauseReasons::default();
        unlock_never_announced.note(SessionLocked);
        assert!(unlock_never_announced.system_can_be_asked_whether_the_reason_still_holds(false));
        let windows_says_the_session_is_unlocked = false;
        unlock_never_announced.session_is_locked = windows_says_the_session_is_unlocked;
        assert_eq!(unlock_never_announced.reason_not_to_render(false), None);
    }

    #[test]
    fn battery_only_pauses_when_asked_to_and_a_manual_pause_outlasts_every_event() {
        let mut pause_reasons = PauseReasons { running_on_battery: true, ..PauseReasons::default() };
        assert_eq!(pause_reasons.reason_not_to_render(false), None);
        assert_eq!(pause_reasons.reason_not_to_render(true), Some("on battery"));
        assert!(pause_reasons.system_can_be_asked_whether_the_reason_still_holds(true));
        assert!(!pause_reasons.system_can_be_asked_whether_the_reason_still_holds(false));

        pause_reasons.paused_by_user = true;
        for event in [SessionUnlocked, DisplayTurnedOn, SystemResumed] {
            pause_reasons.note(event);
        }
        assert_eq!(pause_reasons.reason_not_to_render(false), Some("paused"));
    }
}
