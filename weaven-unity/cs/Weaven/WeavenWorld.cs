using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace Weaven
{
    /// <summary>
    /// High-level C# adapter for the Weaven interaction-topology framework.
    ///
    /// Usage pattern (Unity MonoBehaviour):
    /// <code>
    /// public class GameSimulation : MonoBehaviour
    /// {
    ///     private WeavenWorld _weaven;
    ///
    ///     void Awake()
    ///     {
    ///         _weaven = new WeavenWorld();
    ///         _weaven.LoadSchema(schemaJson);
    ///     }
    ///
    ///     void FixedUpdate()
    ///     {
    ///         _weaven.PushInput(1, "velocity", rb.velocity.magnitude);
    ///         var result = _weaven.Tick();
    ///         foreach (var (smId, change) in result.StateChanges)
    ///         {
    ///             // drive animations, VFX, etc.
    ///         }
    ///     }
    ///
    ///     void OnDestroy() => _weaven.Dispose();
    /// }
    /// </code>
    /// </summary>
    public sealed class WeavenWorld : IDisposable
    {
        private IntPtr _handle;
        private bool _disposed;

        public WeavenWorld()
        {
            _handle = WeavenNative.weaven_create();
            if (_handle == IntPtr.Zero)
                throw new InvalidOperationException("Failed to create WeavenWorld native handle");
        }

        ~WeavenWorld() => Dispose(false);

        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        private void Dispose(bool disposing)
        {
            if (!_disposed && _handle != IntPtr.Zero)
            {
                WeavenNative.weaven_destroy(_handle);
                _handle = IntPtr.Zero;
                _disposed = true;
            }
        }

        private void ThrowIfDisposed()
        {
            if (_disposed) throw new ObjectDisposedException(nameof(WeavenWorld));
        }

        // ── Schema ──────────────────────────────────────────────────────

        /// <summary>
        /// Load a Weaven schema from a JSON string.
        /// </summary>
        /// <exception cref="WeavenException">If the JSON is invalid or fails validation.</exception>
        public void LoadSchema(string json)
        {
            ThrowIfDisposed();
            int rc = WeavenNative.weaven_load_schema(_handle, json);
            if (rc != 0)
                throw new WeavenException("Failed to load schema: invalid or malformed JSON");
        }

        // ── Tick ────────────────────────────────────────────────────────

        /// <summary>
        /// Advance the simulation by one tick.
        /// Returns a <see cref="TickResult"/> with state changes and system commands.
        /// </summary>
        public TickResult Tick()
        {
            ThrowIfDisposed();
            IntPtr jsonPtr = WeavenNative.weaven_tick(_handle);
            // The pointer is owned by the handle (cached until next tick), so just read it.
            string json = Marshal.PtrToStringUTF8(jsonPtr) ?? "{}";
            return TickResult.FromJson(json);
        }

        // ── Input ───────────────────────────────────────────────────────

        /// <summary>
        /// Push a continuous input value into an SM's context field.
        /// Call each frame before <see cref="Tick()"/>.
        /// </summary>
        public void PushInput(uint smId, string field, double value)
        {
            ThrowIfDisposed();
            WeavenNative.weaven_push_input(_handle, smId, field, value);
        }

        /// <summary>
        /// Inject a discrete signal into an SM's input port.
        /// </summary>
        /// <param name="payload">Signal payload as key-value pairs.</param>
        /// <exception cref="WeavenException">If the injection fails.</exception>
        public void InjectSignal(uint smId, uint portId, Dictionary<string, double> payload)
        {
            ThrowIfDisposed();
            string json = DictToJson(payload);
            int rc = WeavenNative.weaven_inject_signal(_handle, smId, portId, json);
            if (rc != 0)
                throw new WeavenException("Failed to inject signal");
        }

        // ── Output ──────────────────────────────────────────────────────

        /// <summary>
        /// Read a context field value from an SM (Continuous Output Port).
        /// </summary>
        public double ReadOutput(uint smId, string field)
        {
            ThrowIfDisposed();
            return WeavenNative.weaven_read_output(_handle, smId, field);
        }

        /// <summary>
        /// Get the active state ID of an SM, or null if the SM doesn't exist.
        /// </summary>
        public int? ActiveState(uint smId)
        {
            ThrowIfDisposed();
            int state = WeavenNative.weaven_active_state(_handle, smId);
            return state >= 0 ? state : null;
        }

        // ── Activation ──────────────────────────────────────────────────

        /// <summary>
        /// Mark an SM for evaluation in the next tick.
        /// </summary>
        public void Activate(uint smId)
        {
            ThrowIfDisposed();
            WeavenNative.weaven_activate(_handle, smId);
        }

        // ── Spatial ─────────────────────────────────────────────────────

        /// <summary>
        /// Enable spatial indexing with the given cell size.
        /// </summary>
        public void EnableSpatial(double cellSize)
        {
            ThrowIfDisposed();
            WeavenNative.weaven_enable_spatial(_handle, cellSize);
        }

        /// <summary>
        /// Update an SM's spatial position (e.g., from Transform).
        /// </summary>
        public void SetPosition(uint smId, double x, double y)
        {
            ThrowIfDisposed();
            WeavenNative.weaven_set_position(_handle, smId, x, y);
        }

        /// <summary>
        /// Query SM IDs within a radius of (x, y).
        /// </summary>
        public uint[] QueryRadius(double x, double y, double radius)
        {
            ThrowIfDisposed();
            IntPtr ptr = WeavenNative.weaven_query_radius(_handle, x, y, radius);
            string json = Marshal.PtrToStringUTF8(ptr) ?? "[]";
            WeavenNative.weaven_free_string(ptr);
            return ParseUintArray(json);
        }

        // ── Snapshot / Restore ──────────────────────────────────────────

        /// <summary>
        /// Take a snapshot of the current world state for rollback networking.
        /// </summary>
        public string TakeSnapshot()
        {
            ThrowIfDisposed();
            IntPtr ptr = WeavenNative.weaven_snapshot(_handle);
            // Pointer is cached by handle; just copy the string.
            return Marshal.PtrToStringUTF8(ptr) ?? "";
        }

        /// <summary>
        /// Restore world state from a snapshot JSON string.
        /// </summary>
        /// <exception cref="WeavenException">If the snapshot is invalid.</exception>
        public void RestoreSnapshot(string snapshotJson)
        {
            ThrowIfDisposed();
            int rc = WeavenNative.weaven_restore(_handle, snapshotJson);
            if (rc != 0)
                throw new WeavenException("Failed to restore snapshot: invalid JSON");
        }

        // ── Utility ─────────────────────────────────────────────────────

        /// <summary>Get the current tick number.</summary>
        public ulong CurrentTick
        {
            get
            {
                ThrowIfDisposed();
                return WeavenNative.weaven_current_tick(_handle);
            }
        }

        /// <summary>Get all registered SM IDs.</summary>
        public uint[] SmIds
        {
            get
            {
                ThrowIfDisposed();
                IntPtr ptr = WeavenNative.weaven_sm_ids(_handle);
                string json = Marshal.PtrToStringUTF8(ptr) ?? "[]";
                WeavenNative.weaven_free_string(ptr);
                return ParseUintArray(json);
            }
        }

        // ── Spawn / Despawn ─────────────────────────────────────────────

        /// <summary>Request spawn of SMs by IDs.</summary>
        public void RequestSpawn(params uint[] smIds)
        {
            ThrowIfDisposed();
            string json = UintArrayToJson(smIds);
            int rc = WeavenNative.weaven_request_spawn(_handle, json);
            if (rc != 0)
                throw new WeavenException("Failed to request spawn");
        }

        /// <summary>Request despawn of SMs by IDs.</summary>
        public void RequestDespawn(params uint[] smIds)
        {
            ThrowIfDisposed();
            string json = UintArrayToJson(smIds);
            int rc = WeavenNative.weaven_request_despawn(_handle, json);
            if (rc != 0)
                throw new WeavenException("Failed to request despawn");
        }

        // ── Network APIs (§8) ─────────────────────────────────────────

        /// <summary>
        /// Compute the diff between two snapshot JSON strings.
        /// Returns a JSON array of state diffs.
        /// </summary>
        /// <exception cref="WeavenException">If either snapshot is invalid.</exception>
        public string DiffSnapshots(string beforeJson, string afterJson)
        {
            ThrowIfDisposed();
            IntPtr ptr = WeavenNative.weaven_diff_snapshots(_handle, beforeJson, afterJson);
            if (ptr == IntPtr.Zero)
                throw new WeavenException("Failed to diff snapshots: invalid JSON");
            string json = Marshal.PtrToStringUTF8(ptr) ?? "[]";
            return json;
        }

        /// <summary>
        /// Register a network policy for an SM.
        /// </summary>
        /// <param name="policyJson">
        /// JSON: {"sm_id":1,"authority":"Server","sync_policy":"StateSync","reconciliation":"Snap"}
        /// </param>
        /// <exception cref="WeavenException">If the policy JSON is invalid.</exception>
        public void SetNetworkPolicy(string policyJson)
        {
            ThrowIfDisposed();
            int rc = WeavenNative.weaven_set_network_policy(_handle, policyJson);
            if (rc != 0)
                throw new WeavenException("Failed to set network policy: invalid JSON");
        }

        /// <summary>
        /// Filter a diff JSON array by registered network policies.
        /// Returns the filtered diff as a JSON string.
        /// </summary>
        /// <exception cref="WeavenException">If the diffs JSON is invalid.</exception>
        public string PolicyFilteredDiff(string diffsJson)
        {
            ThrowIfDisposed();
            IntPtr ptr = WeavenNative.weaven_policy_filtered_diff(_handle, diffsJson);
            if (ptr == IntPtr.Zero)
                throw new WeavenException("Failed to filter diffs: invalid JSON");
            string json = Marshal.PtrToStringUTF8(ptr) ?? "[]";
            return json;
        }

        /// <summary>
        /// Take a scoped snapshot of specific SMs only.
        /// </summary>
        /// <exception cref="WeavenException">If the SM IDs JSON is invalid.</exception>
        public string ScopedSnapshot(params uint[] smIds)
        {
            ThrowIfDisposed();
            string idsJson = UintArrayToJson(smIds);
            IntPtr ptr = WeavenNative.weaven_scoped_snapshot(_handle, idsJson);
            if (ptr == IntPtr.Zero)
                throw new WeavenException("Failed to take scoped snapshot");
            string json = Marshal.PtrToStringUTF8(ptr) ?? "";
            return json;
        }

        /// <summary>
        /// Get SM IDs within a spatial interest region (for network LOD).
        /// </summary>
        public uint[] InterestRegion(float cx, float cy, float radius)
        {
            ThrowIfDisposed();
            IntPtr ptr = WeavenNative.weaven_interest_region(_handle, cx, cy, radius);
            string json = Marshal.PtrToStringUTF8(ptr) ?? "[]";
            WeavenNative.weaven_free_string(ptr);
            return ParseUintArray(json);
        }

        // ── Input Buffer & Rewind ─────────────────────────────────────

        /// <summary>
        /// Initialise the input buffer for rollback networking.
        /// </summary>
        public void InitInputBuffer(uint historyDepth)
        {
            ThrowIfDisposed();
            WeavenNative.weaven_init_input_buffer(_handle, historyDepth);
        }

        /// <summary>
        /// Push a tagged input into the buffer.
        /// </summary>
        /// <param name="inputJson">
        /// JSON: {"tick":0,"target_sm":1,"target_port":0,"payload":{"key":1.0}}
        /// </param>
        /// <exception cref="WeavenException">If the input JSON is invalid or buffer not initialised.</exception>
        public void PushTaggedInput(string inputJson)
        {
            ThrowIfDisposed();
            int rc = WeavenNative.weaven_push_tagged_input(_handle, inputJson);
            if (rc != 0)
                throw new WeavenException("Failed to push tagged input: invalid JSON or buffer not initialised");
        }

        /// <summary>
        /// Apply buffered inputs for the current tick to the world.
        /// </summary>
        /// <exception cref="WeavenException">If the input buffer is not initialised.</exception>
        public void ApplyBufferedInputs()
        {
            ThrowIfDisposed();
            int rc = WeavenNative.weaven_apply_buffered_inputs(_handle);
            if (rc != 0)
                throw new WeavenException("Input buffer not initialised");
        }

        /// <summary>
        /// Save the current world state as the rewind base snapshot.
        /// </summary>
        public void SaveRewindBase()
        {
            ThrowIfDisposed();
            WeavenNative.weaven_save_rewind_base(_handle);
        }

        /// <summary>
        /// Rewind to the saved base snapshot and re-simulate to current tick.
        /// </summary>
        /// <exception cref="WeavenException">If no base snapshot or no input buffer.</exception>
        public void RewindTo(ulong targetTick, ulong currentTick)
        {
            ThrowIfDisposed();
            int rc = WeavenNative.weaven_rewind_to(_handle, targetTick, currentTick);
            if (rc != 0)
                throw new WeavenException("Failed to rewind: no base snapshot or buffer not initialised");
        }

        // ── Helpers ─────────────────────────────────────────────────────

        internal static string DictToJson(Dictionary<string, double> dict)
        {
            var parts = new List<string>(dict.Count);
            foreach (var kv in dict)
            {
                parts.Add($"\"{Escape(kv.Key)}\":{kv.Value:G17}");
            }
            return "{" + string.Join(",", parts) + "}";
        }

        internal static string UintArrayToJson(uint[] ids)
        {
            var parts = new string[ids.Length];
            for (int i = 0; i < ids.Length; i++)
                parts[i] = ids[i].ToString();
            return "[" + string.Join(",", parts) + "]";
        }

        internal static uint[] ParseUintArray(string json)
        {
            // Minimal JSON array parser for "[1,2,3]"
            json = json.Trim();
            if (json.Length <= 2) return Array.Empty<uint>();
            var inner = json.Substring(1, json.Length - 2);
            var parts = inner.Split(',');
            var result = new uint[parts.Length];
            for (int i = 0; i < parts.Length; i++)
                result[i] = uint.Parse(parts[i].Trim());
            return result;
        }

        internal static string Escape(string s)
            => s.Replace("\\", "\\\\").Replace("\"", "\\\"");
    }

    // ── TickResult ──────────────────────────────────────────────────────

    /// <summary>
    /// Result of a single tick, containing state changes and system commands.
    /// </summary>
    public sealed class TickResult
    {
        /// <summary>SM ID → (previous state, new state)</summary>
        public Dictionary<uint, (int Prev, int Next)> StateChanges { get; }
        /// <summary>System commands emitted during this tick (HitStop, SlowMotion, TimeScale).</summary>
        public IReadOnlyList<SystemCommand> SystemCommands { get; }
        public ulong Tick { get; }

        private TickResult(Dictionary<uint, (int, int)> changes, List<SystemCommand> commands, ulong tick)
        {
            StateChanges = changes;
            SystemCommands = commands;
            Tick = tick;
        }

        internal static TickResult FromJson(string json)
        {
            // Minimal parser for the FFI tick output format:
            // {"state_changes":{"1":[0,1]},"system_commands":[...],"tick":1}
            var changes = new Dictionary<uint, (int, int)>();
            var commands = new List<SystemCommand>();
            ulong tick = 0;

            // Extract tick
            int tickIdx = json.IndexOf("\"tick\":", StringComparison.Ordinal);
            if (tickIdx >= 0)
            {
                int start = tickIdx + 6;
                int end = start;
                while (end < json.Length && (char.IsDigit(json[end]) || json[end] == ' '))
                    end++;
                if (ulong.TryParse(json.AsSpan(start, end - start).Trim(stackalloc char[] { ' ' }), out var t))
                    tick = t;
            }

            // Extract state_changes
            int scIdx = json.IndexOf("\"state_changes\":{", StringComparison.Ordinal);
            if (scIdx >= 0)
            {
                int braceStart = json.IndexOf('{', scIdx + 16);
                int depth = 1;
                int pos = braceStart + 1;
                while (pos < json.Length && depth > 0)
                {
                    if (json[pos] == '{') depth++;
                    else if (json[pos] == '}') depth--;
                    pos++;
                }
                string inner = json.Substring(braceStart + 1, pos - braceStart - 2);

                // Parse "1":[0,1],"2":[1,0]
                int i = 0;
                while (i < inner.Length)
                {
                    int keyStart = inner.IndexOf('"', i);
                    if (keyStart < 0) break;
                    int keyEnd = inner.IndexOf('"', keyStart + 1);
                    var key = uint.Parse(inner.AsSpan(keyStart + 1, keyEnd - keyStart - 1));

                    int arrStart = inner.IndexOf('[', keyEnd);
                    int arrEnd = inner.IndexOf(']', arrStart);
                    var arrStr = inner.Substring(arrStart + 1, arrEnd - arrStart - 1);
                    var parts = arrStr.Split(',');
                    int prev = int.Parse(parts[0].Trim());
                    int next = int.Parse(parts[1].Trim());

                    changes[key] = (prev, next);
                    i = arrEnd + 1;
                }
            }

            // Extract system_commands
            int cmdIdx = json.IndexOf("\"system_commands\":[", StringComparison.Ordinal);
            if (cmdIdx >= 0)
            {
                int arrStart = json.IndexOf('[', cmdIdx + 18);
                int depth = 1;
                int pos = arrStart + 1;
                while (pos < json.Length && depth > 0)
                {
                    if (json[pos] == '[') depth++;
                    else if (json[pos] == ']') depth--;
                    pos++;
                }
                string cmdArray = json.Substring(arrStart + 1, pos - arrStart - 2).Trim();
                if (cmdArray.Length > 0)
                    ParseSystemCommands(cmdArray, commands);
            }

            return new TickResult(changes, commands, tick);
        }

        private static void ParseSystemCommands(string cmdArray, List<SystemCommand> commands)
        {
            // Parse [{...},{...},...] at top-level objects
            int i = 0;
            while (i < cmdArray.Length)
            {
                int objStart = cmdArray.IndexOf('{', i);
                if (objStart < 0) break;
                int depth = 1;
                int pos = objStart + 1;
                while (pos < cmdArray.Length && depth > 0)
                {
                    if (cmdArray[pos] == '{') depth++;
                    else if (cmdArray[pos] == '}') depth--;
                    pos++;
                }
                string obj = cmdArray.Substring(objStart, pos - objStart);

                if (obj.Contains("\"HitStop\""))
                {
                    int fIdx = obj.IndexOf("\"frames\":", StringComparison.Ordinal);
                    if (fIdx >= 0)
                    {
                        int s = fIdx + 9;
                        int e = s;
                        while (e < obj.Length && (char.IsDigit(obj[e]) || obj[e] == ' ')) e++;
                        if (uint.TryParse(obj.AsSpan(s, e - s).Trim(), out var frames))
                            commands.Add(new SystemCommand.HitStop(frames));
                    }
                }
                else if (obj.Contains("\"SlowMotion\""))
                {
                    double factor = 1.0;
                    uint durationTicks = 0;
                    int fIdx = obj.IndexOf("\"factor\":", StringComparison.Ordinal);
                    if (fIdx >= 0)
                    {
                        int s = fIdx + 9;
                        int e = s;
                        while (e < obj.Length && (char.IsDigit(obj[e]) || obj[e] == '.' || obj[e] == '-' || obj[e] == ' ')) e++;
                        double.TryParse(obj.AsSpan(s, e - s).Trim(), System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out factor);
                    }
                    int dIdx = obj.IndexOf("\"duration_ticks\":", StringComparison.Ordinal);
                    if (dIdx >= 0)
                    {
                        int s = dIdx + 17;
                        int e = s;
                        while (e < obj.Length && (char.IsDigit(obj[e]) || obj[e] == ' ')) e++;
                        uint.TryParse(obj.AsSpan(s, e - s).Trim(), out durationTicks);
                    }
                    commands.Add(new SystemCommand.SlowMotion(factor, durationTicks));
                }
                else if (obj.Contains("\"TimeScale\""))
                {
                    int tsIdx = obj.IndexOf("\"TimeScale\":", StringComparison.Ordinal);
                    if (tsIdx >= 0)
                    {
                        int s = tsIdx + 12;
                        int e = s;
                        while (e < obj.Length && (char.IsDigit(obj[e]) || obj[e] == '.' || obj[e] == '-' || obj[e] == ' ')) e++;
                        if (double.TryParse(obj.AsSpan(s, e - s).Trim(), System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out var scale))
                            commands.Add(new SystemCommand.TimeScale(scale));
                    }
                }

                i = pos;
            }
        }
    }

    // ── SystemCommand ────────────────────────────────────────────────────

    /// <summary>
    /// System commands emitted by the Weaven tick: HitStop, SlowMotion, TimeScale.
    /// </summary>
    public abstract class SystemCommand
    {
        private SystemCommand() { }

        public sealed class HitStop : SystemCommand
        {
            public uint Frames { get; }
            public HitStop(uint frames) { Frames = frames; }
        }

        public sealed class SlowMotion : SystemCommand
        {
            public double Factor { get; }
            public uint DurationTicks { get; }
            public SlowMotion(double factor, uint durationTicks) { Factor = factor; DurationTicks = durationTicks; }
        }

        public sealed class TimeScale : SystemCommand
        {
            public double Scale { get; }
            public TimeScale(double scale) { Scale = scale; }
        }
    }

    // ── Exception ───────────────────────────────────────────────────────

    public class WeavenException : Exception
    {
        public WeavenException(string message) : base(message) { }
    }
}
