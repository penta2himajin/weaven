import { ReactFlowProvider } from "@xyflow/react";
import TopologyCanvas from "./components/TopologyCanvas";
import TracePanel from "./components/TracePanel";
import InspectorPanel from "./components/InspectorPanel";
import TimelinePanel from "./components/TimelinePanel";
import FilterConfigPanel from "./components/FilterConfigPanel";
import CascadeDetailPanel from "./components/CascadeDetailPanel";
import { useDebugStore } from "./stores/debugStore";

/**
 * Panel layout (design doc §5.4):
 *
 * ┌──────────────────────────────────────────────────┐
 * │  TimelinePanel (tick slider + cascade stepper)   │
 * ├──────────────────────┬───────────────────────────┤
 * │                      │                           │
 * │   TopologyCanvas     │   TracePanel              │
 * │   (React Flow)       │   (signal flow log)       │
 * │                      │                           │
 * │                      ├───────────────────────────┤
 * │                      │                           │
 * │                      │   InspectorPanel          │
 * │                      │   (guard / context)       │
 * │                      │                           │
 * └──────────────────────┴───────────────────────────┘
 */
export default function App() {
  const loaded = useDebugStore((s) => s.loaded);

  if (!loaded) {
    return <WelcomeScreen />;
  }

  return (
    <div className="flex flex-col h-screen w-screen">
      {/* Top bar: Timeline */}
      <TimelinePanel />

      {/* Main area: Canvas + Side panels */}
      <div className="flex flex-1 min-h-0">
        {/* Left 2/3: Topology */}
        <div className="flex-[2] min-w-0 border-r border-gray-800">
          <ReactFlowProvider>
            <TopologyCanvas />
          </ReactFlowProvider>
        </div>

        {/* Right 1/3: Filter + Trace + Inspector stacked */}
        <div className="flex-1 flex flex-col min-w-0">
          <FilterConfigPanel />
          <div className="flex-1 border-b border-gray-800 overflow-auto">
            <TracePanel />
          </div>
          <div className="border-b border-gray-800">
            <CascadeDetailPanel />
          </div>
          <div className="flex-1 overflow-auto">
            <InspectorPanel />
          </div>
        </div>
      </div>
    </div>
  );
}

function WelcomeScreen() {
  return (
    <div className="flex items-center justify-center h-screen">
      <div className="text-center space-y-4 p-12 rounded-xl border-2 border-dashed border-gray-700">
        <h1 className="text-2xl font-bold text-gray-200">Weaven Debugger</h1>
        <p className="text-gray-500 text-sm">
          Drop a weaven-schema JSON file here
        </p>
      </div>
    </div>
  );
}
