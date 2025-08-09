import type { Slide } from "../types";
import { renderContent } from "../utils/dom";
import slideCss from "./slide.css?raw";

/**
 * Custom element for slide
 */
export class TobogganSlideElement extends HTMLElement {
  private root: ShadowRoot;
  private slideContainer: HTMLDivElement | null = null;

  private _slide: Slide | null = null;
  get slide(): Slide | null {
    return this._slide;
  }
  set slide(slide: Slide | null) {
    this._slide = slide;
    if (this.slideContainer) {
      if (!slide) {
        this.slideContainer.className = "empty";
        this.slideContainer.innerHTML = "<p>No slide data available</p>";
      } else {
        let content = "";
        if (slide.title && slide.title.type !== "Empty") {
          content += renderContent(slide.title, "h2");
        }
        if (slide.body && slide.body.type !== "Empty") {
          content += renderContent(slide.body, "div");
        }
        this.slideContainer.innerHTML = content || "<p>Empty slide</p>";
      }
    }
  }

  constructor() {
    super();

    this.root = this.attachShadow({ mode: "open" });

    const style = document.createElement("style");
    style.textContent = slideCss;
    this.root.appendChild(style);
  }

  connectedCallback(): void {
    this.slideContainer = document.createElement("div");
    this.root.appendChild(this.slideContainer);
  }

  disconnectedCallback(): void {
    this.slideContainer = null;
  }
}

// Register the custom element
if (!customElements.get("toboggan-slide")) {
  customElements.define("toboggan-slide", TobogganSlideElement);
}

declare global {
  interface HTMLElementTagNameMap {
    "toboggan-slide": TobogganSlideElement;
  }
}
