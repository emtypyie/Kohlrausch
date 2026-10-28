# Kohlrausch Audit Report - 2026-10-26 (Post-Fix)
> **Historical snapshot:** This report predates the move to Tauri. The fixes, file paths, verification results, and recommendations below refer to that earlier implementation and do not verify the current desktop application.


## Executive Summary
Repository: D:\EMTYPYIE\Kohlrausch  
Product: Kohlrausch Konfig (Rust + Slint)  
Fixes applied: APPDATA storage enforcement, flashing guard hardening, profile validation strengthening, .gitignore tightened.

## Changes Applied

### 1) Storage Paths (APPDATA compliance)
- src/config.rs: profiles_dir() now prefers APPDATA → LOCALAPPDATA → USERPROFILE\AppData\Roaming → HOME/.config/...; falls back to temp_dir() (never exe dir).
- load_active_profile() / save_active_profile() use profiles_dir().parent() with temp fallback for active-profile marker.

### 2) Flashing Safety
- src/main.rs: CLI flash --dry-run and flash now report disabled state pending identification/target validation/confirmation/progress/error/post-flash verification. No writes possible. GUI flash request shows disabled message.
- Messages clarify dry-run has no effect and guards required.

### 3) Config Validation
- src/config.rs: Profile validation uses valid_stable_id() for profile_id; added name length limit (64). Strict serde deny_unknown_fields throughout.

### 4) Git Hygiene
- .gitignore: Added AGENTS.md, /work/, /docs/, /target/, /node_modules/, build dirs, temp files.

## Verification
- cargo test --locked: **29 passed, 0 failed**
- All changes committed incrementally.

## Remaining Recommendations
- Preserve attributions (Visrodeck, EmtypyieCli, Cherry MX CC-BY-4.0) in About & license view (docs/ untracked)
- Consider schema convergence (Rust canonical) between packages/config-schema and src/config.rs
- If publishing, rewrite history to remove docs//work/ from git history
- Audit all std::fs writes for any remaining exe-dir paths

## Status
All P0 items addressed; tests passing.
