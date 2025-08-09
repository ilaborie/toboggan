/**
 * DOM utility functions
 * Provides type-safe DOM element access
 */

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
