// Development-only entry point: renders the production component without the desktop shell.
// Native download commands remain unavailable; no fake success responses are supplied.
import React from "react";
import { createRoot } from "react-dom/client";
import { StoreView } from "../src/components/StoreView";
import "../src/styles.css";

createRoot(document.getElementById("root")!).render(<React.StrictMode><main style={{ display: "flex", height: "100vh", maxWidth: 1100, margin: "auto" }}><StoreView /></main></React.StrictMode>);
