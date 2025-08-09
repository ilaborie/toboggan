import type { Content } from "../types";
import { renderContent } from "../utils/dom";
import footerCss from "./footer.css?raw";

/**
 * Custom element for footer
 */
export class TobogganFooterElement extends HTMLElement {
  private root: ShadowRoot;
  private footerContainer: HTMLElement | null = null;

  private _footer: Content | null = null;
  get footer(): Content | null {
    return this._footer;
  }
  set footer(footer: Content | null) {
    this._footer = footer;
    if (this.footerContainer) {
      if (!footer) {
        this.footerContainer.className = "empty";
        this.footerContainer.innerHTML = "<p>No footer available</p>";
      } else {
        const content = renderContent(footer, "div");
        this.footerContainer.innerHTML = content || "<p>Empty footer</p>";
      }
    }
  }

  constructor() {
    super();

    this.root = this.attachShadow({ mode: "open" });

    const style = document.createElement("style");
    style.textContent = footerCss;
    this.root.appendChild(style);
  }

  connectedCallback(): void {
    this.footerContainer = document.createElement("footer");
    this.root.appendChild(this.footerContainer);
  }

  disconnectedCallback(): void {
    this.footerContainer = null;
  }
}

// Register the custom element
if (!customElements.get("toboggan-footer")) {
  customElements.define("toboggan-footer", TobogganFooterElement);
}

declare global {
  interface HTMLElementTagNameMap {
    "toboggan-footer": TobogganFooterElement;
  }
}
