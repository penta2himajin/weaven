# Mycelia Demo — Setup Guide

## GitHub Actions CI (GameCI)

### 1. Unity License Setup (one-time)

GameCI requires a Unity `.ulf` license file + credentials as GitHub Secrets.

**Get the .ulf file:**

1. Install Unity Hub on a machine with a display
2. Log in and activate a Personal license (Preferences → Licenses)
3. Find the `.ulf` file (it may be in a hidden folder):
   - **Windows**: `C:\ProgramData\Unity\Unity_lic.ulf`
   - **Windows (UAC redirect)**: `C:\Users\<user>\AppData\Local\VirtualStore\ProgramData\Unity\Unity_lic.ulf`
   - **Mac**: `/Library/Application Support/Unity/Unity_lic.ulf`
   - **Linux**: `~/.local/share/unity3d/Unity/Unity_lic.ulf`

> **Tip**: `C:\ProgramData` is a hidden folder. In File Explorer, go to
> View → Show → Hidden items to make it visible.

### 2. Add GitHub Secrets

Go to your repository → Settings → Secrets and variables → Actions, then add:

| Secret Name | Value |
|------------|-------|
| `UNITY_LICENSE` | Full contents of your `.ulf` file (XML text) |
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
