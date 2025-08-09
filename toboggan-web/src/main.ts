/**
 * Toboggan Web Presentation Application
 * Main entry point that initializes and coordinates all modules
 */

import { TobogganApp } from "./app";
import { createAppConfig } from "./config";

import "./components/toast";
import "./components/navigation";
import "./components/footer";
import "./components/slide";

import "./main.css";

// Initialize the application when the DOM is loaded
let app: TobogganApp | null = null;
document.addEventListener("DOMContentLoaded", (): void => {
  const config = createAppConfig();
  app = new TobogganApp(config);
});

// Clean up on page unload
window.addEventListener("beforeunload", () => {
  if (app) {
    app.dispose();
  }
});
