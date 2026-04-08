# Mycelia Demo — Setup Guide

## GitHub Actions CI (GameCI)

### 1. Add GitHub Secrets (one-time)

GameCI v4 handles Personal license activation via credentials directly.
No `.ulf` file is needed.

Go to your repository → Settings → Secrets and variables → Actions, then add:

| Secret Name | Value |
|------------|-------|
| `UNITY_EMAIL` | Your Unity account email |
| `UNITY_PASSWORD` | Your Unity account password |

### 3. Trigger CI

Push to any of these paths to trigger the Mycelia CI pipeline:
- `weaven-core/**`
- `weaven-unity/**`
- `demos/mycelia/**`

The pipeline runs 3 layers:
1. **Rust tests** — `cargo test` (Mycelia schema + full suite)
2. **C# adapter tests** — `dotnet test` (WeavenWorld FFI)
3. **Unity tests + build** — GameCI (EditMode tests + StandaloneLinux64 build)

## Local Development

### Run Rust tests
```bash
cargo test -p weaven-core --test mycelia_demo
```

### Run C# tests
```bash
cargo build -p weaven-unity --release
LD_LIBRARY_PATH=target/release dotnet test weaven-unity/cs/Weaven.Tests/
```

### Run in Unity Editor (requires license + display)
1. Open `demos/mycelia/MyceliaUnity/` in Unity Hub
2. Copy Weaven files:
   ```bash
   # Native library
   cp target/release/libweaven_unity.so demos/mycelia/MyceliaUnity/Assets/Plugins/x86_64/
   # C# adapter
   cp weaven-unity/cs/Weaven/*.cs demos/mycelia/MyceliaUnity/Assets/Scripts/Weaven/
   cp generated/cs/*.cs demos/mycelia/MyceliaUnity/Assets/Scripts/Weaven/
   ```
3. Press Play
