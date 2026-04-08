using System;
using System.Runtime.InteropServices;

namespace Weaven
{
    /// <summary>
    /// P/Invoke declarations for the weaven-unity native plugin (C ABI).
    /// This class is internal — use <see cref="WeavenWorld"/> for the public API.
    /// </summary>
    internal static class WeavenNative
    {
#if UNITY_IOS && !UNITY_EDITOR
        private const string LibName = "__Internal";
#else
        private const string LibName = "weaven_unity";
#endif

        // ── Lifecycle ───────────────────────────────────────────────────

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr weaven_create();

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void weaven_destroy(IntPtr handle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void weaven_free_string(IntPtr ptr);

        // ── Schema ──────────────────────────────────────────────────────

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int weaven_load_schema(IntPtr handle,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string json);

        // ── Tick ────────────────────────────────────────────────────────

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr weaven_tick(IntPtr handle);

        // ── Input ───────────────────────────────────────────────────────

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void weaven_push_input(IntPtr handle, uint smId,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string field, double value);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int weaven_inject_signal(IntPtr handle, uint smId,
            uint portId, [MarshalAs(UnmanagedType.LPUTF8Str)] string payloadJson);

        // ── Output ──────────────────────────────────────────────────────

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern double weaven_read_output(IntPtr handle, uint smId,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string field);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int weaven_active_state(IntPtr handle, uint smId);

        // ── Activation ──────────────────────────────────────────────────

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void weaven_activate(IntPtr handle, uint smId);

        // ── Spatial ─────────────────────────────────────────────────────

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void weaven_enable_spatial(IntPtr handle, double cellSize);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void weaven_set_position(IntPtr handle, uint smId,
            double x, double y);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr weaven_query_radius(IntPtr handle,
            double x, double y, double radius);

        // ── Snapshot / Restore ──────────────────────────────────────────

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr weaven_snapshot(IntPtr handle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int weaven_restore(IntPtr handle,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string json);

        // ── Utility ─────────────────────────────────────────────────────

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong weaven_current_tick(IntPtr handle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr weaven_sm_ids(IntPtr handle);

        // ── Spawn / Despawn ─────────────────────────────────────────────

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int weaven_request_spawn(IntPtr handle,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string smIdsJson);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int weaven_request_despawn(IntPtr handle,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string smIdsJson);

        // ── Network APIs (§8) ─────────────────────────────────────────────

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr weaven_diff_snapshots(IntPtr handle,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string beforeJson,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string afterJson);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int weaven_set_network_policy(IntPtr handle,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string policyJson);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr weaven_policy_filtered_diff(IntPtr handle,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string diffsJson);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr weaven_scoped_snapshot(IntPtr handle,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string smIdsJson);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr weaven_interest_region(IntPtr handle,
            float cx, float cy, float radius);

        // ── Input Buffer & Rewind ─────────────────────────────────────────

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void weaven_init_input_buffer(IntPtr handle,
            uint historyDepth);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int weaven_push_tagged_input(IntPtr handle,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string inputJson);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int weaven_apply_buffered_inputs(IntPtr handle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void weaven_save_rewind_base(IntPtr handle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int weaven_rewind_to(IntPtr handle,
            ulong targetTick, ulong currentTick);
    }
}
