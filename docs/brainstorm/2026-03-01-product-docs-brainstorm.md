---
date: 2026-03-01
topic: product-docs
---

# Product Documentation Brainstorm

## What We're Building

Two product management documents in `docs/product/`:
- `prd.md` — full product requirements document
- `roadmap.md` — versioned milestones and future direction

These are PM-layer documents that sit above the technical brainstorms. They answer WHY and WHAT at the product level; the technical docs answer HOW.

## Key Decisions

- **Structure**: PRD + separate roadmap (not one doc, not per-feature docs)
- **Target users**: Broader Sonos community — not just developers, but any Sonos user who wants a terminal-based experience. Polished onboarding and discoverability matter.
- **v1 success bar**: Full SDK coverage — every SDK capability is accessible from the terminal (CLI or TUI). Coverage is the metric.
- **v2 theme**: Beat the official Sonos app on experience. Win users for more of their daily usage. Not a feature list — a quality and UX bar.
- **TUI positioning**: The TUI is the showpiece — reactive, alive, fun. It should feel better than opening the phone app.

## Resolved Questions

- Single PRD vs. separate initiative docs → PRD + roadmap (two files)
- Target user → Broader community, not personal tool
- v1 bar → Feature completeness / full SDK coverage
- v2 direction → Better experience than official app, more daily usage

## Next Steps

→ Write `docs/product/prd.md` and `docs/product/roadmap.md`
