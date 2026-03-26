import { createContext, useContext, type ReactNode } from "react";
import { createCommands, type TauriInvoke } from "../commands";

type Commands = ReturnType<typeof createCommands>;

const CommandsCtx = createContext<Commands | null>(null);

export function CommandsProvider({
  commands,
  children,
}: {
  commands: Commands;
  children: ReactNode;
}) {
  return <CommandsCtx.Provider value={commands}>{children}</CommandsCtx.Provider>;
}

export function useCommands(): Commands {
  const ctx = useContext(CommandsCtx);
  if (!ctx) throw new Error("useCommands must be used within CommandsProvider");
  return ctx;
}
