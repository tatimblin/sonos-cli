# Milestone 1 Changes - Property Watching with Change Iterator

## Summary

Restored the original property watching interface with change iterator that drives reactive rendering. The TUI reacts to events from the iterator, not by polling state. Milestone 1 defines the interface with stub implementations; milestone 2 delivers actual events.

## What's Included

### Query Interface (for initial state and on-demand lookups)
- `SpeakerInfo` / `GroupInfo` structs (plain data, no SDK handles)
- `SpeakerState` / `GroupState` structs (current state snapshots)
- `query_speaker()` / `query_group()` functions
- `list_all_speakers()` / `list_all_groups()` functions

### Property Watching Interface (for reactive updates)
- `PropertyKey` enum (Volume, Mute, Bass, Treble, Loudness, PlaybackState, Position, CurrentTrack, GroupMembership, GroupVolume, GroupMute)
- `watch_property()` / `unwatch_property()` functions
- `change_events() -> &ChangeIterator` - the key reactive primitive
- **Milestone 1**: watch/unwatch are no-ops, iterator is empty
- **Milestone 2**: Actual event delivery from SDK

## TUI Usage Pattern

```rust
// TUI maintains only UI state, not data
struct TuiState {
    current_view: View,
    selected_index: usize,
    scroll_offset: usize,
}

// TUI event loop (future milestone)
loop {
    select! {
        // Terminal input
        event = terminal_events.next() => {
            if let Some(action) = handle_input(event) {
                executor::execute(action, &system, &config)?;
            }
        }
        
        // Property changes from SDK - drives reactive rendering
        change = executor::change_events(&system).next() => {
            if let Some(change) = change {
                // Change event triggers re-render
                // All data comes from SonosSystem, not stored in TUI
                render_ui(&system, &tui_state);
            }
        }
    }
}

// Rendering queries SonosSystem state directly
fn render_ui(system: &SonosSystem, tui_state: &TuiState) {
    let groups = executor::list_all_groups(system);
    let selected_group = executor::query_group(system, &groups[tui_state.selected_index].coordinator_name)?;
    
    // Render using data from SonosSystem
    draw_group_info(&selected_group);
    draw_volume_bar(selected_group.volume);
    // ...
}
```

**State management:**
- **SonosSystem stores all speaker/group data** - single source of truth
- TUI stores only UI state (current view, selection, scroll position)
- Change events trigger re-render, which queries SonosSystem for current data
- Query functions provide access to SonosSystem state

## The Key Technical Piece

**Change iterator causes re-renders on change events** - this is the reactive primitive. The TUI doesn't poll state, it reacts to the iterator yielding events. When a property changes, the iterator yields and the TUI re-renders by querying SonosSystem state.

**TUI is a stateless wrapper** - all speaker/group data lives in SonosSystem. The TUI only stores UI concerns (view, selection, scroll).

## Architecture Preserved

- **Executor is the single SDK interaction point**
- CLI and TUI never import or call SDK types directly
- Query functions provide state snapshots
- Change iterator provides reactive updates
- Both work through executor

## What This Enables

1. **Milestone 1**: Define interfaces, stub implementations (no-ops, empty iterator)
2. **Milestone 2**: Implement actual event delivery from SDK
3. **TUI**: React to change iterator, render on events

The interface exists now so TUI code can be written correctly from the start. No refactoring needed when events are implemented.


