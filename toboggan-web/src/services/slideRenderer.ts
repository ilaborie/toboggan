/**
 * Slide Renderer Service
 * Handles rendering of slide content to the DOM
 */

import type { Content, Slide } from "../types.js";
import { escapeHtml } from "../utils/dom.js";

export class SlideRenderer {
  private readonly appElement: HTMLElement;

  constructor(appElement: HTMLElement) {
    this.appElement = appElement;
  }

  /**
   * Display a slide in the app element
   */
  public displaySlide(slide: Slide | null): void {
    if (!slide) {
      this.appElement.innerHTML = "<p>No slide data available</p>";
      return;
    }

    let content = "";

    // Add slide title if present
    if (slide.title && slide.title.type !== "Empty") {
      content += this.renderContent(slide.title, "h2");
    }

    // Add slide body
    if (slide.body && slide.body.type !== "Empty") {
      content += this.renderContent(slide.body, "div");
    }

    // Add slide notes if present (maybe for accessibility)
    if (slide.notes && slide.notes.type !== "Empty") {
      content += "<details><summary>Notes</summary>";
      content += this.renderContent(slide.notes, "div");
      content += "</details>";
    }

    this.appElement.innerHTML = content || "<p>Empty slide</p>";
  }

  /**
   * Clear the slide display
   */
  public clear(): void {
    this.appElement.innerHTML = "<p>Loading presentation...</p>";
  }

  /**
   * Render content based on its type
   */
  private renderContent(content: Content, wrapper: string = "div"): string {
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
  }
}
