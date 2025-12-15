import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "@promptgen/ui";
import { DesktopBackendProvider } from "./backend/desktop";
import { init as initWasm } from "@promptgen/core-wasm";
import "./index.css";

// Initialize WASM module before rendering
initWasm().then(() => {
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <DesktopBackendProvider>
        <App />
      </DesktopBackendProvider>
    </StrictMode>
  );
});
