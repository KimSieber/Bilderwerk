import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import Ueber from "./components/Ueber";
import "./styles.css";

// Ein Frontend für alle Fenster: „Über“ öffnet index.html?fenster=ueber (v0.3)
const fenster = new URLSearchParams(window.location.search).get("fenster");

createRoot(document.getElementById("root")!).render(<StrictMode>{fenster === "ueber" ? <Ueber /> : <App />}</StrictMode>);
