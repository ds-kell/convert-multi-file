import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { windowLabel } from "./api";
import App from "./App";
import QuickApp from "./QuickApp";
import "./styles.css";

const root = document.getElementById("root");
if (root === null) throw new Error("#root element missing from index.html");

// The backend opens the "quick" window for Explorer context-menu conversions;
// every other window shows the full app.
const View = windowLabel === "quick" ? QuickApp : App;

createRoot(root).render(
  <StrictMode>
    <View />
  </StrictMode>,
);
