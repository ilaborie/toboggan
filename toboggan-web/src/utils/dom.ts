/**
 * DOM utility functions
 * Provides type-safe DOM element access
 */


/**
 * Escape HTML to prevent XSS attacks
 */
export function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

export function injectStyle(node: Element | ShadowRoot, css?: string) {
  node.innerHTML += `
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.cyan.min.css">
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.colors.min.css">
`;

  if (css) {
    const style = document.createElement("style");
    style.textContent = css;
    node.appendChild(style);
  }
}