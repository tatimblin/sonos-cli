# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.0](https://github.com/tatimblin/sonos-cli/compare/v0.9.1...v0.10.0) - 2026-11-22

### Changed

- let the SDK keep the playback position current ([#120](https://github.com/tatimblin/sonos-cli/pull/120))

## [0.9.0](https://github.com/tatimblin/sonos-cli/compare/v0.8.0...v0.9.0) - 2026-11-22

### Added

- adopt sonos-sdk 0.10 ([#117](https://github.com/tatimblin/sonos-cli/pull/117))

### Changed

- share selection resolvers between screens and handlers ([#111](https://github.com/tatimblin/sonos-cli/pull/111))
- split speaker list rows into per-variant data and renderers ([#110](https://github.com/tatimblin/sonos-cli/pull/110))
- drive TUI key handling and legend from one binding table ([#109](https://github.com/tatimblin/sonos-cli/pull/109))
- move read-command formatting into format.rs ([#107](https://github.com/tatimblin/sonos-cli/pull/107))
- resolve speaker-or-group targets in one place ([#106](https://github.com/tatimblin/sonos-cli/pull/106))
- use the library crate from the binary and move config commands into cli ([#97](https://github.com/tatimblin/sonos-cli/pull/97))

### Fixed

- fetch playback position so the progress bar moves ([#115](https://github.com/tatimblin/sonos-cli/pull/115))
- match speaker and group completions case-insensitively ([#114](https://github.com/tatimblin/sonos-cli/pull/114))
- detect discovery failures by error variant instead of message text ([#108](https://github.com/tatimblin/sonos-cli/pull/108))
- reject out-of-range volume, bass and treble as usage errors ([#105](https://github.com/tatimblin/sonos-cli/pull/105))
- keep the cursor on the same speaker when topology changes ([#104](https://github.com/tatimblin/sonos-cli/pull/104))
- interpolate progress between position updates ([#103](https://github.com/tatimblin/sonos-cli/pull/103))
- complete group names for --group ([#102](https://github.com/tatimblin/sonos-cli/pull/102))
- append to the log file instead of truncating it ([#101](https://github.com/tatimblin/sonos-cli/pull/101))
- make alias names case-insensitive when setting ([#100](https://github.com/tatimblin/sonos-cli/pull/100))
- refuse to overwrite a config file that fails to parse ([#99](https://github.com/tatimblin/sonos-cli/pull/99))

## [0.8.0](https://github.com/tatimblin/sonos-cli/compare/v0.7.2...v0.8.0) - 2026-11-21

### Added

- add mute toggle and indicator to the speaker list ([#94](https://github.com/tatimblin/sonos-cli/pull/94))

## [0.7.2](https://github.com/tatimblin/sonos-cli/compare/v0.7.1...v0.7.2) - 2026-11-21

### Added

- add dynamic shell completions for speakers, groups, and aliases ([#91](https://github.com/tatimblin/sonos-cli/pull/91))

### Fixed

- *(deps)* bump sonos-sdk to 0.9.0 ([#92](https://github.com/tatimblin/sonos-cli/pull/92))

## [0.7.0](https://github.com/tatimblin/sonos-cli/compare/v0.6.0...v0.7.0) - 2026-09-18

### Fixed

- stop reporting total discovery failure as success ([#83](https://github.com/tatimblin/sonos-cli/pull/83))

## [0.6.0](https://github.com/tatimblin/sonos-cli/compare/v0.5.0...v0.6.0) - 2026-08-18

### Added

- [**breaking**] upgrade to sonos-sdk 0.7.1 ([#77](https://github.com/tatimblin/sonos-cli/pull/77))

## [0.5.0](https://github.com/tatimblin/sonos-cli/compare/v0.4.0...v0.5.0) - 2026-05-23

### Added

- add speaker/group aliases with -s/-g short flags ([#70](https://github.com/tatimblin/sonos-cli/pull/70))

## [0.4.0](https://github.com/tatimblin/sonos-cli/compare/v0.3.0...v0.4.0) - 2026-05-03

### Added

- *(tui)* theme overhaul, gradient progress bar, and mini-player fixes ([#62](https://github.com/tatimblin/sonos-cli/pull/62))

## [0.3.0](https://github.com/tatimblin/sonos-cli/compare/v0.2.0...v0.3.0) - 2026-05-02

### Added

- *(tui)* drop zone regrouping mode for speaker pick-up ([#58](https://github.com/tatimblin/sonos-cli/pull/58))
- *(tui)* add persistent bottom player bar with playback controls ([#56](https://github.com/tatimblin/sonos-cli/pull/56))
- *(tui)* enhanced speaker list with 2-line group headers and tree connectors ([#57](https://github.com/tatimblin/sonos-cli/pull/57))
- *(tui)* add Settings tab with theme, default group, and album art controls ([#55](https://github.com/tatimblin/sonos-cli/pull/55))

### Changed

- *(tui)* separation of concerns — widgets, screens, handlers ([#54](https://github.com/tatimblin/sonos-cli/pull/54))

### Fixed

- uri fallback display for missing track metadata ([#60](https://github.com/tatimblin/sonos-cli/pull/60))

## [0.2.0](https://github.com/tatimblin/sonos-cli/compare/v0.1.2...v0.2.0) - 2026-04-05

### Added

- *(tui)* shared SpeakerList widget with pick-up/drop regrouping ([#45](https://github.com/tatimblin/sonos-cli/pull/45))
- *(tui)* add album art and Now Playing tab ([#44](https://github.com/tatimblin/sonos-cli/pull/44))

### Fixed

- align sonos-sdk dev-dependency version with main dependency

## [0.1.2](https://github.com/tatimblin/sonos-cli/compare/v0.1.1...v0.1.2) - 2026-03-29

### Added

- *(tui)* implement Home Screen with live group cards and speaker management ([#29](https://github.com/tatimblin/sonos-cli/pull/29))
- *(tui)* scaffold TUI foundation with event loop, navigation, and theme ([#22](https://github.com/tatimblin/sonos-cli/pull/22))

### Changed

- *(tui)* replace global widget state with hooks architecture ([#35](https://github.com/tatimblin/sonos-cli/pull/35))
- Migrate TUI to WatchHandle RAII API with logging and event fixes ([#34](https://github.com/tatimblin/sonos-cli/pull/34))
- *(tui)* migrate to WatchHandle RAII API ([#32](https://github.com/tatimblin/sonos-cli/pull/32))

### Fixed

- *(ci)* disable release-plz GitHub Release creation ([#20](https://github.com/tatimblin/sonos-cli/pull/20))

## [0.1.1](https://github.com/tatimblin/sonos-cli/compare/v0.1.0...v0.1.1) - 2026-03-22

### Added

- add platform-specific discovery failure diagnostics ([#17](https://github.com/tatimblin/sonos-cli/pull/17))
# Changelog
