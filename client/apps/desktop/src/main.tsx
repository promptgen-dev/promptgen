import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "@promptgen/ui";
import { DesktopBackendProvider } from "./backend/desktop";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <DesktopBackendProvider>
      <App />
    </DesktopBackendProvider>
  </StrictMode>
);
