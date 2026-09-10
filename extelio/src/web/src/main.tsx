import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "./design/tokens.css";
import "./design/typography.css";
import "./design/components/base.css";
import "./design/components/controls.css";
import "./design/components/data.css";
import "./design/components/feedback.css";

import { App } from "./App";

const container = document.getElementById("root");
if (!container) {
  throw new Error("Wurzelelement nicht gefunden");
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
