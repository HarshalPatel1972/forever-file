# Forever File - Project Development Journal

> A comprehensive log of the development journey, including all steps, errors, and solutions.

---

## 📅 Project Timeline

| Date | Phase | Status |
|------|-------|--------|
| 2026-01-12 | Phase 0: Initialization | ✅ Complete |
| 2026-01-12 | Phase 1: Core Engine (magic-wormhole) | ❌ Failed |
| 2026-01-13 | Phase 1: Core Engine (iroh) | ✅ Complete |

---

## Phase 0: Project Initialization

### Step 1: Git Repository Setup
```bash
git init
git branch -M main
```
- Created `.gitignore` to exclude build artifacts, `node_modules/`, `target/`, etc.

### Step 2: Project IDX Environment Configuration
Created `.idx/dev.nix` with NixOS configuration:
- Rust toolchain (`rustup`)
- Node.js 20
- GTK/WebKit dependencies for Tauri

### Step 3: Tauri Project Scaffold
```bash
npm create tauri-app@latest . -- --template react-ts --manager npm
```
- Package name: `forever-file`
- Identifier: `com.harshal-patel.tauri-appforever-file`

### Step 4: Initial Commits
```bash
git commit -m "chore(infra): add idx environment config"
git commit -m "chore(init): bootstrap tauri v2 environment in idx"
```

**✅ Phase 0 Result: Success**

---

## Phase 1: Core Engine Implementation

### Attempt 1: magic-wormhole (FAILED)

#### Step 1: Add Dependencies
Updated `src-tauri/Cargo.toml`:
```toml
magic-wormhole = "0.6"
tokio = { version = "1", features = ["full"] }
futures = "0.3"
thiserror = "2"
```

#### Step 2: Implement TransferManager
Created `src-tauri/src/wormhole_transfer.rs` with:
- `send_file()` function
- `receive_file()` function
- Unit test for local transfer

#### Error 1: Tauri `bundle` Feature Conflict
```
package `tauri-appforever-file` depends on `tauri` with feature `bundle` 
but `tauri` does not have that feature.
```
**Solution:** Removed `bundle` feature from `tauri` dependency in Cargo.toml.

#### Error 2: Missing MSVC Linker (Windows)
```
error: linker `link.exe` not found
```
**Solution:** This is a Windows-specific issue. Need Visual Studio Build Tools. Decided to test in Firebase Studio (Linux) instead.

#### Error 3: Missing `.idx/dev.nix` in Remote
Firebase Studio cloned an empty repo because `.idx/dev.nix` was in `.gitignore`.
**Solution:** Force-added `.idx/dev.nix` to git:
```bash
git add -f .idx/dev.nix
git commit -m "fix(infra): restore .idx/dev.nix for idx environment"
```

#### Error 4: Missing GTK Dependencies
```
pkg-config exited with status code 1
The system library `glib-2.0` required by crate `glib-sys` was not found.
```
**Solution:** Updated `.idx/dev.nix` to include ALL GTK dependencies:
```nix
pkgs.glib
pkgs.openssl
pkgs.zlib
pkgs.cairo
pkgs.pango
pkgs.gdk-pixbuf
pkgs.atk
```

#### Error 5: Firebase Studio Stale Environment
Even after updating `dev.nix`, the terminal wasn't picking up the new packages.
**Diagnosis:** `PKG_CONFIG_PATH=/usr/lib/pkgconfig` (wrong) instead of `/nix/store/...` (correct).
**Solution:** Needed to "Rebuild Environment" but command was not available in Firebase Studio.

#### Error 6: Nix Package Conflicts
```bash
nix-env -iA nixpkgs.gtk3 nixpkgs.librsvg...
```
```
error: Unable to build profile. There is a conflict for the following files:
    .../librsvg-2.57.0/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache
    .../gdk-pixbuf-2.42.10/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache
```
**Solution:** Used `nix-shell` instead of `nix-env` to avoid conflicts.

#### Error 7: magic-wormhole API Mismatch
The code I wrote was based on an outdated API. Version 0.6 has completely different function signatures:
```
error[E0432]: unresolved import `magic_wormhole::transfer::receive_file`
error[E0433]: could not find `wormhole` in `magic_wormhole`
error[E0308]: mismatched types - expected `AppConfig<_>`, found `AppID`
```
**Solution:** Abandoned magic-wormhole entirely.

---

### Attempt 2: iroh (SUCCESS)

#### Step 1: Research Alternatives
Searched for simpler P2P crates. Found `iroh`:
- Modern, well-documented API
- QUIC-based with built-in hole-punching
- No GTK/GUI dependencies

#### Step 2: Create Standalone Test Crate
Created `iroh-test/` directory with minimal dependencies:
```toml
[dependencies]
iroh = "0.95"
iroh-ping = "0.7"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

#### Error 8: Wrong iroh Version
```
error: failed to select a version for the requirement `iroh = "^0.37"`
candidate versions found which didn't match: 0.95.1, 0.94.0, 0.93.2, ...
```
**Solution:** Updated to `iroh = "0.95"`.

#### Error 9: Wrong iroh-ping Version
```
error: failed to select a version for the requirement `iroh-ping = "^0.95"`
candidate versions found which didn't match: 0.7.0, 0.6.0, 0.5.0, ...
```
**Solution:** iroh-ping has different versioning. Updated to `iroh-ping = "0.7"`.

#### Error 10: Disk Space Full
```
error: No space left on device (os error 28)
```
**Solution:** Cleared build caches:
```bash
rm -rf ~/forever-file/src-tauri/target
rm -rf ~/forever-file/wormhole-test/target
rm -rf ~/.cargo/registry/cache
```

#### Final Test: SUCCESS! 🎉
```
=== Iroh P2P Connectivity Test ===
Receiver address: EndpointAddr { id: PublicKey(...), addrs: {...} }
Sending ping...
✅ SUCCESS! Ping completed in 1.937847ms
P2P connection established and verified!
```

**✅ Phase 1 Result: Success (with iroh)**

---

## Phase 2: Tauri Bridge (Interop)

### Goal
Expose the P2P transfer logic to the React frontend without freezing the UI using an **Event-Driven Architecture**.

### Implementation

#### Step 1: Create Feature Branch
```bash
git checkout feat/core-engine
git checkout -b feat/tauri-bridge
```

#### Step 2: Update Dependencies
Replaced `magic-wormhole` with `iroh` in `src-tauri/Cargo.toml`:
```toml
iroh = "0.95"
iroh-blobs = "0.35"
tokio = { version = "1", features = ["full"] }
```

#### Step 3: Implement Event-Driven Commands (`lib.rs`)
Key architecture decisions:
- **`tokio::spawn`** for non-blocking background tasks
- **`app.emit()`** to send events to frontend
- **`app.clone()`** before moving into async threads

Events implemented:
| Event | Payload | Purpose |
|-------|---------|---------|
| `forever-file://ticket-generated` | `{ ticket: string }` | Share connection ticket |
| `forever-file://progress` | `{ sent: number, total: number }` | Transfer progress |
| `forever-file://complete` | `{ success: boolean, message: string }` | Transfer finished |
| `forever-file://error` | `{ message: string }` | Error occurred |

#### Step 4: Create TypeScript Types (`src/types.ts`)
```typescript
export interface TicketGeneratedEvent { ticket: string; }
export interface ProgressEvent { sent: number; total: number; }
export interface TransferCompleteEvent { success: boolean; message: string; }
export interface ErrorEvent { message: string; }
```

#### Step 5: Add Frontend Listeners (`App.tsx`)
- Added `useEffect` with `listen()` for all events
- Created test button to invoke `start_send`
- Live event log display

#### Step 6: Commit
```bash
git commit -m "feat(bridge): implement event-driven tauri commands for async file transfer"
git push origin feat/tauri-bridge
```

**✅ Phase 2 Result: Implementation Complete (Pending Verification)**

---

## Next Steps (Phase 3)
```
forever-file/
├── .idx/
│   └── dev.nix           # NixOS environment config
├── src/                   # React frontend
├── src-tauri/             # Tauri backend (Rust)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   └── wormhole_transfer.rs  # (deprecated - magic-wormhole)
├── iroh-test/             # Standalone P2P test crate ✅
│   ├── Cargo.toml
│   └── src/main.rs
├── wormhole-test/         # (deprecated - failed approach)
├── package.json
└── PROJECT_README.md      # This file
```

---

## Key Learnings

1. **Tauri v2 has heavy dependencies** - Requires full GTK stack on Linux.
2. **magic-wormhole 0.6 API is different from docs** - The crate's README is outdated.
3. **Firebase Studio ≠ Project IDX** - Different environment management.
4. **iroh is excellent** - Modern, well-documented, minimal dependencies.
5. **nix-shell > nix-env** - Avoids package conflicts.
6. **Cloud environments have disk limits** - Clean build caches regularly.

---

## Next Steps (Phase 2)

- [ ] Integrate iroh into `src-tauri` for file transfer
- [ ] Build Tauri frontend UI
- [ ] Implement send/receive file functionality

---

*Last Updated: 2026-01-13*
