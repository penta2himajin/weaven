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

        // ── Helpers ─────────────────────────────────────────────────────

        private static string DictToJson(Dictionary<string, double> dict)
        {
            var parts = new List<string>(dict.Count);
            foreach (var kv in dict)
            {
                parts.Add($"\"{Escape(kv.Key)}\":{kv.Value:G17}");
            }
            return "{" + string.Join(",", parts) + "}";
        }

        private static string UintArrayToJson(uint[] ids)
        {
            var parts = new string[ids.Length];
            for (int i = 0; i < ids.Length; i++)
                parts[i] = ids[i].ToString();
            return "[" + string.Join(",", parts) + "]";
        }

        private static uint[] ParseUintArray(string json)
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

        private static string Escape(string s)
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
        public ulong Tick { get; }

        private TickResult(Dictionary<uint, (int, int)> changes, ulong tick)
        {
            StateChanges = changes;
            Tick = tick;
        }

        internal static TickResult FromJson(string json)
        {
            // Minimal parser for the FFI tick output format:
            // {"state_changes":{"1":[0,1]},"system_commands":[...],"tick":1}
            var changes = new Dictionary<uint, (int, int)>();
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

            return new TickResult(changes, tick);
        }
    }

    // ── Exception ───────────────────────────────────────────────────────

    public class WeavenException : Exception
    {
        public WeavenException(string message) : base(message) { }
    }
}
