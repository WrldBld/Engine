//! Game Time System (Phase 23C)
//!
//! In-game time tracking for sessions. Used for:
//! - LLM cache TTL (cache invalidates when game time advances)
//! - Story progression
//! - NPC scheduling context (day/night)
//!
//! Game time is DM-controlled by default (time_scale = 0), meaning
//! time only advances when the DM explicitly advances it.

use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// Time of day for NPC scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeOfDay {
    Morning,   // 6:00 - 11:59
    Afternoon, // 12:00 - 17:59
    Evening,   // 18:00 - 21:59
    Night,     // 22:00 - 5:59
}

impl TimeOfDay {
    /// Get the display name for this time of day
    pub fn display_name(&self) -> &'static str {
        match self {
            TimeOfDay::Morning => "Morning",
            TimeOfDay::Afternoon => "Afternoon",
            TimeOfDay::Evening => "Evening",
            TimeOfDay::Night => "Night",
        }
    }

    /// Get the icon identifier for this time of day
    pub fn icon(&self) -> &'static str {
        match self {
            TimeOfDay::Morning => "sun_rising",
            TimeOfDay::Afternoon => "sun",
            TimeOfDay::Evening => "sunset",
            TimeOfDay::Night => "moon",
        }
    }
}

impl std::fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// In-game time tracking for a session
///
/// # Design Notes
///
/// Game time is decoupled from real time:
/// - `time_scale = 0.0` means time is paused (DM manually advances)
/// - `time_scale = 1.0` means 1 real second = 1 game second
/// - `time_scale > 1.0` means time passes faster than real time
///
/// The default is paused (0.0), giving the DM full control over
/// when time passes in the story.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameTime {
    /// Current in-game date and time
    pub current: DateTime<Utc>,
    
    /// How fast game time passes relative to real time
    /// - 0.0 = paused (default, DM controls time)
    /// - 1.0 = real-time
    /// - >1.0 = accelerated
    pub time_scale: f32,
    
    /// Last real-world time we updated game time
    /// Used to calculate elapsed time when time_scale > 0
    pub last_updated: DateTime<Utc>,
}

impl Default for GameTime {
    fn default() -> Self {
        Self::new()
    }
}

impl GameTime {
    /// Create a new game time, starting at the current real-world time
    /// with time paused (DM controls advancement)
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            current: now,
            time_scale: 0.0,
            last_updated: now,
        }
    }

    /// Create a new game time starting at a specific date/time
    pub fn starting_at(start: DateTime<Utc>) -> Self {
        Self {
            current: start,
            time_scale: 0.0,
            last_updated: Utc::now(),
        }
    }

    /// Advance game time by a fixed amount (DM action)
    pub fn advance(&mut self, duration: Duration) {
        self.current = self.current + duration;
        self.last_updated = Utc::now();
    }

    /// Advance game time by hours
    pub fn advance_hours(&mut self, hours: u32) {
        self.advance(Duration::hours(hours as i64));
    }

    /// Advance game time by days
    pub fn advance_days(&mut self, days: u32) {
        self.advance(Duration::days(days as i64));
    }

    /// Set a new time (for jumping to specific story moments)
    pub fn set_time(&mut self, new_time: DateTime<Utc>) {
        self.current = new_time;
        self.last_updated = Utc::now();
    }

    /// Set the time scale (0 = paused, 1 = realtime, etc.)
    pub fn set_time_scale(&mut self, scale: f32) {
        // Before changing scale, sync current time if we were running
        if self.time_scale > 0.0 {
            self.sync();
        }
        self.time_scale = scale.max(0.0);
        self.last_updated = Utc::now();
    }

    /// Sync game time with elapsed real time (only matters if time_scale > 0)
    pub fn sync(&mut self) {
        if self.time_scale > 0.0 {
            let now = Utc::now();
            let real_elapsed = now - self.last_updated;
            let game_elapsed_seconds = real_elapsed.num_milliseconds() as f64 
                * (self.time_scale as f64) / 1000.0;
            self.current = self.current + Duration::milliseconds(game_elapsed_seconds as i64);
            self.last_updated = now;
        }
    }

    /// Get the current game time (syncs if running)
    pub fn now(&mut self) -> DateTime<Utc> {
        self.sync();
        self.current
    }

    /// Get the current game time without syncing (for display only)
    pub fn current(&self) -> DateTime<Utc> {
        self.current
    }

    /// Get time of day for NPC scheduling
    pub fn time_of_day(&self) -> TimeOfDay {
        let hour = self.current.hour();
        match hour {
            6..=11 => TimeOfDay::Morning,
            12..=17 => TimeOfDay::Afternoon,
            18..=21 => TimeOfDay::Evening,
            _ => TimeOfDay::Night,
        }
    }

    /// Get the current game day (1-based day counter since start)
    pub fn day_number(&self) -> u32 {
        self.current.ordinal()
    }

    /// Get a human-readable time string
    pub fn display_time(&self) -> String {
        let hour = self.current.hour();
        let minute = self.current.minute();
        let period = if hour >= 12 { "PM" } else { "AM" };
        let display_hour = if hour == 0 {
            12
        } else if hour > 12 {
            hour - 12
        } else {
            hour
        };
        format!("{}:{:02} {}", display_hour, minute, period)
    }

    /// Get a human-readable date string
    pub fn display_date(&self) -> String {
        format!(
            "Day {}, {}",
            self.day_number(),
            self.display_time()
        )
    }

    /// Check if game time is paused
    pub fn is_paused(&self) -> bool {
        self.time_scale == 0.0
    }

    /// Calculate hours elapsed since a given game time
    pub fn hours_since(&self, other: &DateTime<Utc>) -> f64 {
        let duration = self.current - *other;
        duration.num_minutes() as f64 / 60.0
    }

    /// Calculate days elapsed since a given game time
    pub fn days_since(&self, other: &DateTime<Utc>) -> f64 {
        let duration = self.current - *other;
        duration.num_hours() as f64 / 24.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_game_time_is_paused() {
        let gt = GameTime::new();
        assert!(gt.is_paused());
        assert_eq!(gt.time_scale, 0.0);
    }

    #[test]
    fn test_advance_hours() {
        let mut gt = GameTime::new();
        let initial = gt.current;
        gt.advance_hours(6);
        assert_eq!((gt.current - initial).num_hours(), 6);
    }

    #[test]
    fn test_advance_days() {
        let mut gt = GameTime::new();
        let initial = gt.current;
        gt.advance_days(2);
        assert_eq!((gt.current - initial).num_days(), 2);
    }

    #[test]
    fn test_time_of_day() {
        // Morning (10:00)
        let morning = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T10:00:00Z").unwrap().into()
        );
        assert_eq!(morning.time_of_day(), TimeOfDay::Morning);

        // Afternoon (14:00)
        let afternoon = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T14:00:00Z").unwrap().into()
        );
        assert_eq!(afternoon.time_of_day(), TimeOfDay::Afternoon);

        // Evening (20:00)
        let evening = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T20:00:00Z").unwrap().into()
        );
        assert_eq!(evening.time_of_day(), TimeOfDay::Evening);

        // Night (2:00)
        let night = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T02:00:00Z").unwrap().into()
        );
        assert_eq!(night.time_of_day(), TimeOfDay::Night);
    }

    #[test]
    fn test_display_time() {
        let gt = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T14:30:00Z").unwrap().into()
        );
        assert_eq!(gt.display_time(), "2:30 PM");

        let gt_morning = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T09:15:00Z").unwrap().into()
        );
        assert_eq!(gt_morning.display_time(), "9:15 AM");
    }

    #[test]
    fn test_hours_since() {
        let gt = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T14:00:00Z").unwrap().into()
        );
        let past = DateTime::parse_from_rfc3339("2024-01-01T10:00:00Z").unwrap().into();
        assert_eq!(gt.hours_since(&past), 4.0);
    }
}
