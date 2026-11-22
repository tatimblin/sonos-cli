//! TUI hooks system — co-located widget state, SDK subscriptions, and animation.
//!
//! Modeled after React hooks but adapted for Rust's ownership model and
//! ratatui's immediate-mode rendering. Three primitives:
//!
//! - `use_state<V>(key, default)` — Persistent local state across renders
//! - `use_watch(property_handle)` — Subscribe to SDK property, return current value
//! - `use_animation(key, active)` — Request periodic re-renders when active
//!
//! `use_watch` acquires one SDK `WatchHandle` per property on first access and
//! keeps it across frames. `WatchHandle` is a live view — `value()` re-reads the
//! state store — so reading through the cached handle is all a later frame needs.
//!
//! ## Calling Convention
//!
//! `use_state` returns `&mut V` which borrows `&mut self` on `Hooks`.
//! To avoid borrow conflicts, call hooks in this order:
//!
//! 1. `use_watch` — returns owned `Option<V>`, borrow released immediately
//! 2. `use_animation` — `&mut self` borrow released immediately
//! 3. `use_state` — must be called last or in a scoped block
//!
//! ## Frame Lifecycle
//!
//! ```text
//! hooks.begin_frame()     // reset access tracking
//! terminal.draw(...)      // widgets call hooks
//! hooks.end_frame()       // evict unaccessed state + handles
//! ```

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use sonos_sdk::property::{GroupFetchable, GroupPropertyHandle, PropertyHandle, WatchHandle};
use sonos_sdk::SdkError;
use sonos_sdk::SonosProperty;

use crate::tui::app::App;

// ============================================================================
// RenderContext
// ============================================================================

/// Render context passed to all render functions.
///
/// Wraps `&App` (read) and `&mut Hooks` (write), satisfying the borrow checker
/// by separating immutable app data from mutable hook state.
pub struct RenderContext<'a> {
    pub app: &'a App,
    pub hooks: &'a mut Hooks,
}

// ============================================================================
// HookKey — state identity
// ============================================================================

/// Composite key for `use_state`: combines value type with a string name.
///
/// Two hooks with the same string key but different value types get separate
/// storage slots (keyed by `TypeId`).
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct HookKey {
    type_id: TypeId,
    name: String,
}

impl HookKey {
    fn new<V: 'static>(name: &str) -> Self {
        Self {
            type_id: TypeId::of::<V>(),
            name: name.to_string(),
        }
    }
}

// ============================================================================
// ProgressState — moved from app.rs to co-locate with hooks
// ============================================================================

/// Longest wall-clock gap between two updates that counts as playback time.
///
/// Frames arrive every ~250ms while playing, so a longer gap means the process
/// was suspended (system sleep, a stalled terminal). Capping each gap limits
/// how far the bar can drift before the SDK reports a fresh position.
const MAX_FRAME_GAP: Duration = Duration::from_secs(10);

/// Client-side progress interpolation state for a single group.
///
/// The SDK only reports a new position on transport events, so between them
/// the bar is advanced locally. `update` is called every frame with the SDK's
/// cached values: it re-anchors only when those values change (a seek, track
/// change or fresh position report) and otherwise accumulates wall-clock time
/// while playing. Pausing freezes the accumulated position; resuming continues
/// from it.
#[derive(Clone, Debug)]
pub struct ProgressState {
    /// SDK position at the last re-anchor.
    anchor_position_ms: u64,
    /// Playing time accumulated since the anchor, up to `last_tick`.
    elapsed_ms: u64,
    /// Wall-clock time of the last `update`.
    last_tick: Instant,
    duration_ms: u64,
    is_playing: bool,
    /// Last `(position_ms, duration_ms)` seen from the SDK, to detect changes.
    sdk_snapshot: Option<(u64, u64)>,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self {
            anchor_position_ms: 0,
            elapsed_ms: 0,
            last_tick: Instant::now(),
            duration_ms: 0,
            is_playing: false,
            sdk_snapshot: None,
        }
    }
}

impl ProgressState {
    /// Update from the SDK's cached position and playback state as of `now`.
    pub fn update(&mut self, position_ms: u64, duration_ms: u64, is_playing: bool, now: Instant) {
        // Credit time since the last update under the *previous* playing state,
        // so a pause freezes at the position reached by `now`.
        self.elapsed_ms = self.elapsed_at(now);
        self.last_tick = now;

        if self.sdk_snapshot != Some((position_ms, duration_ms)) {
            self.sdk_snapshot = Some((position_ms, duration_ms));
            self.anchor_position_ms = position_ms;
            self.elapsed_ms = 0;
        }

        self.duration_ms = duration_ms;
        self.is_playing = is_playing;
    }

    /// Compute the interpolated position in milliseconds as of `now`.
    pub fn interpolated_position_ms(&self, now: Instant) -> u64 {
        (self.anchor_position_ms + self.elapsed_at(now)).min(self.duration_ms)
    }

    fn elapsed_at(&self, now: Instant) -> u64 {
        if !self.is_playing {
            return self.elapsed_ms;
        }
        let gap = now
            .saturating_duration_since(self.last_tick)
            .min(MAX_FRAME_GAP);
        self.elapsed_ms + gap.as_millis() as u64
    }
}

// ============================================================================
// Hooks
// ============================================================================

/// General-purpose hooks system for TUI widget state management.
///
/// Stores persistent state, SDK watch handles, and animation registrations.
/// Uses mark-and-sweep to automatically clean up state when widgets stop
/// rendering (e.g., screen transitions).
pub struct Hooks {
    // use_state storage — keyed by (TypeId, name)
    states: HashMap<HookKey, Box<dyn Any>>,

    // use_watch storage — type-erased WatchHandle<P>, keyed by "speaker_id:property_key"
    watches: HashMap<String, Box<dyn Any>>,

    // use_animation — keys of active animations
    animations: HashSet<String>,

    // Mark-and-sweep: keys accessed during current frame
    accessed_states: HashSet<HookKey>,
    accessed_watches: HashSet<String>,
    accessed_animations: HashSet<String>,
}

impl Default for Hooks {
    fn default() -> Self {
        Self::new()
    }
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            watches: HashMap::new(),
            animations: HashSet::new(),
            accessed_states: HashSet::new(),
            accessed_watches: HashSet::new(),
            accessed_animations: HashSet::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Frame lifecycle
    // -----------------------------------------------------------------------

    /// Reset access tracking before a render frame.
    pub fn begin_frame(&mut self) {
        self.accessed_states.clear();
        self.accessed_watches.clear();
        self.accessed_animations.clear();
    }

    /// Evict unaccessed state and drop unaccessed watch handles.
    ///
    /// Handles for widgets that rendered this frame are retained, so a
    /// continuously-visible widget holds one subscription for its whole life.
    pub fn end_frame(&mut self) {
        // Evict unaccessed states
        self.states
            .retain(|key, _| self.accessed_states.contains(key));

        // Drop unaccessed watch handles (starts grace periods)
        self.watches
            .retain(|key, _| self.accessed_watches.contains(key));

        // Remove unaccessed animations
        self.animations
            .retain(|key| self.accessed_animations.contains(key));
    }

    // -----------------------------------------------------------------------
    // use_state
    // -----------------------------------------------------------------------

    /// Get or create persistent local state.
    ///
    /// On first call for a given key, creates the state using `default()`.
    /// On subsequent calls, returns the existing state.
    ///
    /// **Must be called last** — the returned `&mut V` borrows `&mut self`,
    /// preventing other hook calls until the reference is dropped.
    pub fn use_state<V: 'static>(&mut self, key: &str, default: impl FnOnce() -> V) -> &mut V {
        let hook_key = HookKey::new::<V>(key);
        self.accessed_states.insert(hook_key.clone());

        self.states
            .entry(hook_key)
            .or_insert_with(|| Box::new(default()))
            .downcast_mut::<V>()
            .expect("use_state: type mismatch for key (same key used with different types)")
    }

    // -----------------------------------------------------------------------
    // use_watch
    // -----------------------------------------------------------------------

    /// Get-or-create a cached `WatchHandle` and read its current value.
    ///
    /// `WatchHandle` is a live view: `value()` reads the state store on every
    /// call, so one handle acquired on the first frame reports every subsequent
    /// change. The handle is therefore created once, parked in `self.watches`,
    /// and only *read* on later frames — no per-frame `watch()` call, no
    /// drop/grace-period/re-acquire churn.
    ///
    /// `acquire` is only invoked on a cache miss. `fallback` supplies a value
    /// when acquisition fails (a plain cache read via `get()`), and is not
    /// cached, so a transient failure is retried on the next frame.
    fn use_watch_cached<P>(
        &mut self,
        key: String,
        acquire: impl FnOnce() -> Result<WatchHandle<P>, SdkError>,
        fallback: impl FnOnce() -> Option<P>,
    ) -> Option<P>
    where
        P: 'static,
    {
        self.accessed_watches.insert(key.clone());

        let handle = match self.watches.entry(key) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => match acquire() {
                Ok(wh) => e.insert(Box::new(wh)),
                Err(err) => {
                    tracing::warn!("use_watch: watch() failed ({err}), falling back to get()");
                    return fallback();
                }
            },
        };

        handle
            .downcast_ref::<WatchHandle<P>>()
            .expect("use_watch: same key used with two property types")
            .value()
    }

    /// Subscribe to an SDK speaker property, returning its current value.
    ///
    /// The subscription is acquired on first access and held until the widget
    /// stops rendering (mark-and-sweep in [`Self::end_frame`]).
    pub fn use_watch<P>(&mut self, prop: &PropertyHandle<P>) -> Option<P>
    where
        P: SonosProperty + Clone + 'static,
    {
        let key = format!("{}:{}", prop.speaker_id(), P::KEY);
        self.use_watch_cached(key, || prop.watch(), || prop.get())
    }

    /// Subscribe to an SDK group property, returning its current value.
    ///
    /// Same as `use_watch` but for group-scoped properties (e.g., group volume).
    #[allow(dead_code)]
    pub fn use_watch_group<P>(&mut self, prop: &GroupPropertyHandle<P>) -> Option<P>
    where
        P: SonosProperty + Clone + 'static,
    {
        let key = format!("group:{}:{}", prop.group_id(), P::KEY);
        self.use_watch_cached(key, || prop.watch(), || prop.get())
    }

    /// Subscribe to a fetchable group property, fetching on first access if empty.
    ///
    /// Like `use_watch_group` but performs a one-time fetch when the cache has
    /// no value yet, so the UI doesn't show a blank until the first UPnP event.
    /// The fetch happens only on the frame that acquires the handle, since
    /// later frames reuse it.
    pub fn use_watch_group_or_fetch<P>(&mut self, prop: &GroupPropertyHandle<P>) -> Option<P>
    where
        P: SonosProperty + GroupFetchable + Clone + 'static,
    {
        let key = format!("group:{}:{}", prop.group_id(), P::KEY);
        self.use_watch_cached(key, || prop.watch_or_fetch(), || prop.get())
    }

    // -----------------------------------------------------------------------
    // use_animation
    // -----------------------------------------------------------------------

    /// Register an animation tick request.
    ///
    /// When `active` is true, the event loop's global animation timer will
    /// mark the app as dirty every ~250ms, triggering re-renders for smooth
    /// progress bar animation.
    pub fn use_animation(&mut self, key: &str, active: bool) {
        let key = key.to_string();
        self.accessed_animations.insert(key.clone());
        if active {
            self.animations.insert(key);
        } else {
            self.animations.remove(&key);
        }
    }

    /// Check if any widget has registered an active animation.
    ///
    /// Called by the event loop between frames to decide whether to tick
    /// the animation timer.
    pub fn has_active_animations(&self) -> bool {
        !self.animations.is_empty()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// A `use_watch` handle acquired on one frame must report values written
    /// *after* that frame, without being re-acquired.
    ///
    /// This is the property that made the per-frame re-`watch()` pattern
    /// obsolete (sonos-sdk PR #99). If `WatchHandle` ever reverts to a frozen
    /// snapshot, `use_watch` would silently pin the first observed value and the
    /// TUI would stop updating — a failure with no compiler signal at all, which
    /// is why it is asserted here rather than trusted from the SDK's own suite.
    #[test]
    fn use_watch_reports_values_written_after_the_handle_was_acquired() {
        use sonos_sdk::{SonosSystem, Volume};

        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let speaker = system.speakers().into_iter().next().expect("one speaker");
        let speaker_id = speaker.id.clone();

        let mut hooks = Hooks::new();

        // Frame 1: acquires the handle. Nothing observed yet.
        hooks.begin_frame();
        assert_eq!(hooks.use_watch(&speaker.volume), None);
        hooks.end_frame();

        system
            .state_manager()
            .set_property(&speaker_id, Volume::new(42));

        // Frame 2: same cached handle, new value.
        hooks.begin_frame();
        assert_eq!(
            hooks.use_watch(&speaker.volume).map(|v| v.value()),
            Some(42),
            "a handle held across frames must read the store live"
        );
        hooks.end_frame();

        system
            .state_manager()
            .set_property(&speaker_id, Volume::new(43));

        // Frame 3: keeps tracking, rather than refreshing once.
        hooks.begin_frame();
        assert_eq!(
            hooks.use_watch(&speaker.volume).map(|v| v.value()),
            Some(43)
        );
        hooks.end_frame();
    }

    /// The handle is acquired once and reused, not rebuilt per frame.
    ///
    /// Asserts the churn reduction directly: `watches` must hold exactly one
    /// entry for the property across many frames, and mark-and-sweep must still
    /// evict it once the widget stops rendering.
    #[test]
    fn use_watch_reuses_one_handle_across_frames_then_evicts() {
        use sonos_sdk::SonosSystem;

        let system = SonosSystem::with_speakers(&["Kitchen"]);
        let speaker = system.speakers().into_iter().next().expect("one speaker");

        let mut hooks = Hooks::new();

        // Identity of the stored handle, so a re-acquire is detectable — a
        // count-only assertion passes even if the handle is dropped and rebuilt
        // every frame, which is exactly the churn this migration removed.
        let mut first_addr: Option<usize> = None;

        for frame in 0..5 {
            hooks.begin_frame();
            let _ = hooks.use_watch(&speaker.volume);
            hooks.end_frame();

            assert_eq!(
                hooks.watches.len(),
                1,
                "one property watched continuously must hold exactly one handle"
            );

            let addr = hooks
                .watches
                .values()
                .next()
                .map(|b| std::ptr::addr_of!(**b) as *const () as usize)
                .expect("handle present");

            match first_addr {
                None => first_addr = Some(addr),
                Some(expected) => assert_eq!(
                    addr, expected,
                    "frame {frame} replaced the handle — it must be acquired once and reused"
                ),
            }
        }

        // Widget stops rendering → handle released.
        hooks.begin_frame();
        hooks.end_frame();
        assert!(
            hooks.watches.is_empty(),
            "unaccessed handle must be evicted"
        );
    }

    #[test]
    fn use_state_creates_default_on_first_call() {
        let mut hooks = Hooks::new();
        hooks.begin_frame();

        let val = hooks.use_state::<u32>("counter", || 42);
        assert_eq!(*val, 42);
    }

    #[test]
    fn use_state_persists_across_frames() {
        let mut hooks = Hooks::new();

        // Frame 1: create state
        hooks.begin_frame();
        *hooks.use_state::<u32>("counter", || 0) = 10;
        hooks.end_frame();

        // Frame 2: state persists
        hooks.begin_frame();
        let val = hooks.use_state::<u32>("counter", || 0);
        assert_eq!(*val, 10);
        hooks.end_frame();
    }

    #[test]
    fn use_state_evicts_on_unaccessed_frame() {
        let mut hooks = Hooks::new();

        // Frame 1: create state
        hooks.begin_frame();
        *hooks.use_state::<u32>("counter", || 42) = 100;
        hooks.end_frame();

        // Frame 2: state NOT accessed → evicted
        hooks.begin_frame();
        hooks.end_frame();

        // Frame 3: recreated from default
        hooks.begin_frame();
        let val = hooks.use_state::<u32>("counter", || 42);
        assert_eq!(*val, 42); // default, not 100
    }

    #[test]
    fn use_state_different_types_same_name_are_separate() {
        let mut hooks = Hooks::new();
        hooks.begin_frame();

        *hooks.use_state::<u32>("val", || 1) = 10;
        // Borrow on hooks released after this line since u32 is Copy

        let s = hooks.use_state::<String>("val", || "hello".to_string());
        assert_eq!(s, "hello"); // separate slot, not confused with u32
    }

    #[test]
    fn use_animation_registers_and_deregisters() {
        let mut hooks = Hooks::new();
        hooks.begin_frame();

        assert!(!hooks.has_active_animations());

        hooks.use_animation("progress", true);
        assert!(hooks.has_active_animations());

        hooks.use_animation("progress", false);
        assert!(!hooks.has_active_animations());
    }

    #[test]
    fn use_animation_evicts_on_unaccessed_frame() {
        let mut hooks = Hooks::new();

        // Frame 1: register animation
        hooks.begin_frame();
        hooks.use_animation("progress", true);
        hooks.end_frame();
        assert!(hooks.has_active_animations());

        // Frame 2: animation NOT accessed → evicted
        hooks.begin_frame();
        hooks.end_frame();
        assert!(!hooks.has_active_animations());
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// Repeating the same SDK position every frame must not reset the clock —
    /// the bar keeps advancing between position reports.
    #[test]
    fn progress_advances_while_sdk_position_is_unchanged() {
        let t0 = Instant::now();
        let mut ps = ProgressState::default();

        ps.update(1_000, 60_000, true, t0);
        ps.update(1_000, 60_000, true, t0 + ms(250));
        ps.update(1_000, 60_000, true, t0 + ms(500));

        assert_eq!(ps.interpolated_position_ms(t0 + ms(500)), 1_500);
        assert_eq!(ps.interpolated_position_ms(t0 + ms(750)), 1_750);
    }

    #[test]
    fn progress_reanchors_when_sdk_position_changes() {
        let t0 = Instant::now();
        let mut ps = ProgressState::default();

        ps.update(1_000, 60_000, true, t0);
        ps.update(1_000, 60_000, true, t0 + ms(2_000));
        assert_eq!(ps.interpolated_position_ms(t0 + ms(2_000)), 3_000);

        // Seek: a new SDK position wins over the interpolated one.
        ps.update(30_000, 60_000, true, t0 + ms(2_250));
        assert_eq!(ps.interpolated_position_ms(t0 + ms(2_250)), 30_000);
        assert_eq!(ps.interpolated_position_ms(t0 + ms(3_250)), 31_000);
    }

    #[test]
    fn progress_reanchors_on_track_change_at_the_same_position() {
        let t0 = Instant::now();
        let mut ps = ProgressState::default();

        ps.update(0, 60_000, true, t0);
        ps.update(0, 60_000, true, t0 + ms(5_000));
        // The next track also reports 0:00, but with a different duration.
        ps.update(0, 90_000, true, t0 + ms(5_000));

        assert_eq!(ps.interpolated_position_ms(t0 + ms(5_000)), 0);
    }

    #[test]
    fn progress_freezes_on_pause_and_resumes_on_play() {
        let t0 = Instant::now();
        let mut ps = ProgressState::default();

        ps.update(1_000, 60_000, true, t0);
        ps.update(1_000, 60_000, false, t0 + ms(2_000));
        assert_eq!(ps.interpolated_position_ms(t0 + ms(2_000)), 3_000);

        // Stays frozen across paused frames.
        ps.update(1_000, 60_000, false, t0 + ms(9_000));
        assert_eq!(ps.interpolated_position_ms(t0 + ms(12_000)), 3_000);

        // Resumes from the frozen position, not from the stale SDK value.
        ps.update(1_000, 60_000, true, t0 + ms(12_000));
        assert_eq!(ps.interpolated_position_ms(t0 + ms(13_000)), 4_000);
    }

    #[test]
    fn progress_caps_at_duration() {
        let t0 = Instant::now();
        let mut ps = ProgressState::default();

        ps.update(4_900, 5_000, true, t0);
        assert_eq!(ps.interpolated_position_ms(t0 + ms(1_000)), 5_000);
    }

    /// A long gap between updates (system sleep) counts for at most
    /// `MAX_FRAME_GAP`, while ordinary frames keep accumulating past it.
    #[test]
    fn progress_caps_each_frame_gap() {
        let t0 = Instant::now();
        let mut ps = ProgressState::default();

        ps.update(0, 600_000, true, t0);
        ps.update(0, 600_000, true, t0 + ms(60_000));
        assert_eq!(ps.interpolated_position_ms(t0 + ms(60_000)), 10_000);

        let mut t = t0 + ms(60_000);
        for _ in 0..80 {
            t += ms(250);
            ps.update(0, 600_000, true, t);
        }
        assert_eq!(ps.interpolated_position_ms(t), 30_000);
    }

    #[test]
    fn mark_and_sweep_preserves_accessed_drops_rest() {
        let mut hooks = Hooks::new();

        // Frame 1: create multiple states
        hooks.begin_frame();
        *hooks.use_state::<u32>("keep", || 1) = 10;
        // Release borrow
        let _ = hooks.use_state::<u32>("drop_me", || 2);
        hooks.use_animation("keep_anim", true);
        hooks.use_animation("drop_anim", true);
        hooks.end_frame();

        // Frame 2: only access "keep" variants
        hooks.begin_frame();
        let val = hooks.use_state::<u32>("keep", || 99);
        assert_eq!(*val, 10); // persisted
        hooks.use_animation("keep_anim", true);
        hooks.end_frame();

        // Frame 3: verify "drop_me" was evicted
        hooks.begin_frame();
        let val = hooks.use_state::<u32>("drop_me", || 99);
        assert_eq!(*val, 99); // recreated from default
        hooks.end_frame();

        // And "drop_anim" should be gone
        // (only "keep_anim" should remain)
    }
}
