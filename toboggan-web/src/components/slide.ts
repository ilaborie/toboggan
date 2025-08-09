import type { Content, Slide } from "../types";
import slideCss from "./slide.css?raw";

/**
 * Escape HTML to prevent XSS attacks
 */
const escapeHtml = (text: string): string => {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
};

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
/**
 * Render content based on its type
 */
export const renderContent = (content: Content, wrapper: string = "div"): string => {
  if (!content) return "";

  switch (content.type) {
    case "Text":
      return `<${wrapper}>${escapeHtml(content.text)}</${wrapper}>`;

    case "Html":
      // Note: In a production app, you'd want to sanitize this HTML
      return `<${wrapper}>${content.raw}</${wrapper}>`;

    case "Md":
      // Simple markdown rendering - just display as text for now
      // In a full implementation, you'd use a markdown parser
      return `<${wrapper}><pre>${escapeHtml(content.content)}</pre></${wrapper}>`;

    case "IFrame":
      return `<${wrapper}><iframe src="${escapeHtml(content.url)}" title="${content.alt || "Embedded content"}"></iframe></${wrapper}>`;

    case "Empty":
      return "";

    default:
      return `<${wrapper}>Unsupported content type: ${(content as { type: string }).type}</${wrapper}>`;
  }
};
// Register the custom element
if (!customElements.get("toboggan-slide")) {
  customElements.define("toboggan-slide", TobogganSlideElement);
}

declare global {
  interface HTMLElementTagNameMap {
    "toboggan-slide": TobogganSlideElement;
  }
}
