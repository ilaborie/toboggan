/**
 * DOM utility functions
 * Provides type-safe DOM element access
 */

import type { Content } from "../types";

// export function injectStyle(node: Element | ShadowRoot, css?: string) {
//   node.innerHTML += `
//   <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.cyan.min.css">
//   <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.colors.min.css">
// `;

//   if (css) {
//     const style = document.createElement("style");
//     style.textContent = css;
//     node.appendChild(style);
//   }
// }

export const getRequireElement = <E extends HTMLElement>(
  selector: string,
  parent: ParentNode = document
): E => {
  const result = parent.querySelector(selector);
  if (!result) {
    throw new Error(`missing ${selector} element`);
  }
  return result as E;
};

/**
 * Escape HTML to prevent XSS attacks
 */
const escapeHtml = (text: string): string => {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
};

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
