---
title: "feat: Show home theater satellites and control their settings"
status: active
created: 2026-11-22
depth: standard
milestones:
  - "Milestone 4: CLI — Volume, EQ & Grouping (new: home theater commands)"
  - "Milestone 7: TUI — Home Screen (Speakers tab: satellite rows)"
---

# feat: Show home theater satellites and control their settings

## Summary

A home theater soundbar with bonded surrounds or a sub shows up today as one
speaker; the SDK filters the satellites out of discovery and drops what the
topology says about them. This plan makes satellites visible in the TUI as a
third level under their soundbar, puts the sub and surround levels on those
rows, and adds sub, surround, night mode and speech enhancement commands to
the CLI. A dedicated settings page, opened with Enter on a satellite row, is
left for later.

```
▶ Basement ::::: Wildflowers — Tom Petty ::::::::::::::::::: 25%
  └─ Basement • Sonos Playbar                                25%
     ├─ Left surround • Sonos One              ■■■■■··· ♪ +3  TV +8
     └─ Right surround • Sonos One                      ♪ +3  TV +8
```

Satellites can't be picked up: they move with their soundbar, as they do in
Sonos.

## Live evidence (Basement, 2026-11-22)

Probed read-only from the user's terminal (`GetZoneGroupState`, `GetEQ`,
`device_description.xml`):

- **Main:** `RINCON_5CAAFDAE58BD01400`, Sonos Playbar (S9), 192.168.5.62,
  firmware 86.2-70230 (17.2.2).
- **Its `HTSatChanMapSet`:**
  `RINCON_5CAAFDAE58BD01400:LF,RF;RINCON_7828CA128F0001400:LR;RINCON_7828CAFB9D9C01400:RR`
- **Satellites:** both `Invisible="1"`, both Sonos One.
  - `…128F00` at 192.168.5.14 is LR (left rear).
  - `…FB9D9C` at 192.168.4.217 is RR (right rear).
- **No sub.**
- **Each satellite's own `HTSatChanMapSet`** lists only the main and itself.
  The main member's map is the complete one and is the source of truth.
- **`GetEQ` on the Playbar:**

  | EQType | Value |
  |---|---|
  | SubEnable | 1 |
  | SubGain | 0 |
  | SubPolarity | 0 |
  | SubCrossover | 0 |
  | SurroundEnable | 1 |
  | SurroundLevel | 8 |
  | MusicSurroundLevel | 0 |
  | SurroundMode | 1 |
  | NightMode | 0 |
  | DialogLevel | 1 |
  | SpeechEnhanceEnabled | 0 |
  | AudioDelay | 0 |
  | AudioDelayLeftRear | 0 |
  | AudioDelayRightRear | 0 |
  | HeightChannelLevel | 0 |

**Consequence:** the soundbar answers every EQType, sub ones included, with
no sub attached. Whether a setting applies has to come from the topology
(a sub row needs an `SW` channel, height needs height hardware), never from
`GetEQ` succeeding.

**Resolved:** `DialogLevel=1` and `SpeechEnhanceEnabled=0` disagree, and the
Sonos app shows Speech Enhancement on for Basement, so `DialogLevel` is the
toggle on the Playbar. Newer soundbars (Arc Ultra) may use the second one.

## PR 1 — SDK: home theater topology

**sonos-state:**

- **Channel map type:** parse the visible member's `HTSatChanMapSet` into
  `HomeTheater { satellites: Vec<SatelliteInfo { id, role }> }`.
- **Roles:** `SatelliteRole` is `LeftSurround` (LR), `RightSurround` (RR),
  `Sub` (SW) or `Other(String)` for anything unrecognised, so a new channel
  code doesn't fail the parse. The main's own `LF,RF` entry is not a
  satellite. A set with two subs lists `SW` twice; keep both.
- **Store:** keep the map per main speaker, replacing the flat
  `satellite_ids` set (derive that set from it, so the exclusion logic in
  `system.rs` keeps working). Update it on every topology event, so
  adding or removing a sub in the app shows up live.
- **Fill the existing field:** `model::Speaker::satellites: Vec<SpeakerId>`
  is never populated today; populate it or drop it in favour of the new map.

**sonos-sdk:**

- **Remember satellite devices:** `rebuild_speakers_excluding_satellites`
  should keep the excluded devices (model name, IP) instead of discarding
  them.
- **Public API:** `Speaker::satellites() -> Vec<Satellite>`, where
  `Satellite { id, role, model_name, ip }`. It reads live state on each
  call and is empty for a speaker that isn't a home theater.
- **Tests:** decoder test using the Basement channel map above verbatim,
  a sub-and-two-surrounds map, and an unknown channel code.
- **Docs:** satellites section in the website guide and
  `docs/references/sonos-sdk.md` (CLI repo).

## PR 2 — SDK: home theater settings

New per-speaker properties on the soundbar, using RenderingControl
`GetEQ`/`SetEQ` with an `EQType`. Each one is `Fetchable` and `Settable`
(the generic `set()` from #125). They should also be evented if
`LastChange` carries them; see the check below.

| Property | EQType | Type / range |
|---|---|---|
| `SubEnabled` | SubEnable | bool |
| `SubLevel` | SubGain | i8, −15..=15 |
| `SurroundEnabled` | SurroundEnable | bool |
| `SurroundLevel` (TV) | SurroundLevel | i8, −15..=15 |
| `MusicSurroundLevel` | MusicSurroundLevel | i8, −15..=15 |
| `SurroundMode` | SurroundMode | enum: `Ambient` = 0, `Full` = 1 |
| `NightMode` | NightMode | bool |
| `SpeechEnhancement` | DialogLevel | bool |
| `AudioDelay` | AudioDelay | u8, 0..=5 (range to confirm) |

**sonos-api:**

- **Operations:** `GetEQ`/`SetEQ` operations. Ranges are validated in
  `Validate`, matching bass and treble.
- **Event parsing:** add the elements above to `RenderingControlInstance`.
  They come as `<SubGain val="…"/>` etc. inside `LastChange` (assumed; see
  the live check).

**Live check before merging:**

- **Do changes arrive as events?** Run the SDK's watch example against
  Basement from the user's terminal, change the surround level and night
  mode in the Sonos app, and confirm `ChangeEvent`s arrive.
- **If they don't:** these become fetch-only and the TUI re-fetches them
  when a satellite row is selected, like position before #132.
- **Confirm the values match the app** (expected: TV level +8, music
  level 0).

Leave out for now: `SubPolarity`, `SubCrossover`, the per-rear audio
delays, `HeightChannelLevel`. These are niche, and the Playbar has no
height hardware to test against.

## PR 3 — CLI

Built on sonos-sdk 0.11.0 (PRs 1 and 2, released together). The design was
revised from the first draft after a mockup review:

**TUI satellite rows:**

- **Entry:** `ListEntry::SatelliteRow { main, satellite }`, after its
  speaker's row, in role order. Hidden during pick-up.
- **Row:** `Left surround • Sonos One`, nested one level under the speaker,
  with a `│` continuation when the speaker isn't last in its group (new
  `connector_pipe` glyph).
- **Values:** a surround row shows both levels, `♪ +3  TV +8`. When selected,
  the level being edited gets an 8-column bar and the accent colour; the other
  stays a muted number. A sub row shows its one level. Switched-off hardware
  shows `off`.
- **Keys** (`SATELLITE_KEYS`, its own `Mode::Satellite`, so the legend reads
  `←→ Level  ⇥ ♪/TV  m On/Off`):
  - `←→` steps the edited level by 1.
  - `⇥` switches between music and TV. The choice lives in
    `SpeakerListScreenState::level_focus`, so it carries to the other surround.
  - `m` toggles `SurroundEnabled` / `SubEnabled`.
  - `␣` shows "Satellites move with Basement" instead of picking up.
  - `p`/`n`/`b` act on the group.
- **Enter** does nothing yet: it will open a dedicated home theater settings
  page (night mode, speech enhancement, surround mode, audio delay) in a later
  PR. No Settings tab rows.

**CLI commands**, all speaker-only like `bass` (`--group` is a usage error):

- `sonos sub <-15..15|on|off>`
- `sonos surround <-15..15|on|off> [--tv]` (music level by default; `--tv`
  with on/off is a usage error)
- `sonos night <on|off>`, `sonos speech <on|off>`
- `sub` and `surround` check `satellites()` and fail with
  `CliError::Unsupported` ("Basement has no sub", exit 1), since a soundbar
  accepts those settings with no hardware bonded.
- `sonos status` lists satellites under a home theater speaker.

## Risks

- **Undocumented EQTypes:** they come from community projects (SoCo,
  node-sonos-http-api), not from Sonos. The live probe confirms they exist
  on the Playbar; ranges are not confirmed.
- **Only one setup to test:** with no sub, the sub row and `sonos sub` are
  verified only by unit tests built from known channel maps.
- **Subs and satellites cover more than home theaters.** A sub can also
  bond to a plain speaker, such as a One plus a Sub. The topology marks it
  the same way, so the design handles it, but the Settings gating must not
  assume a soundbar.
