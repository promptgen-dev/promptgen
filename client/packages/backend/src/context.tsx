import { createContext, useContext, type ReactNode } from "react";
import type { PromptgenBackend } from "./interface";

const BackendContext = createContext<PromptgenBackend | null>(null);

export interface BackendProviderProps {
  backend: PromptgenBackend;
  children: ReactNode;
}

export function BackendProvider({ backend, children }: BackendProviderProps) {
  return (
    <BackendContext.Provider value={backend}>
      {children}
    </BackendContext.Provider>
  );
}

export function useBackend(): PromptgenBackend {
  const backend = useContext(BackendContext);
  if (!backend) {
    throw new Error("useBackend must be used within a BackendProvider");
  }
  return backend;
}
