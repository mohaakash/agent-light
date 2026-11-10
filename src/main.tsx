import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app/App";
import { TrafficLightWindow } from "./windows/traffic-light/TrafficLightWindow";
import "./styles/global.css";

const isTrafficLightWindow = new URLSearchParams(window.location.search).get("window") === "traffic-light";

document.body.className = isTrafficLightWindow ? "traffic-light-host" : "dashboard-host";
document.documentElement.className = isTrafficLightWindow ? "traffic-light-host" : "dashboard-host";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {isTrafficLightWindow ? <TrafficLightWindow /> : <App />}
  </StrictMode>,
);
