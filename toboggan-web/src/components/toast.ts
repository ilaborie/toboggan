import { getRequireElement } from "../utils/dom";
import toastCss from "./toast.css?raw";

/**
 * Toast Web Component
 * Native web component for individual toast notifications with shadow DOM encapsulation
 */

export type ToastType = "error" | "warning" | "info" | "success";

/**
 * Custom element for individual toast messages
 */
export class TobogganToastElement extends HTMLElement {
  private root: ShadowRoot;
  private toastContainer: HTMLDivElement | null = null;

  constructor() {
    super();

    this.root = this.attachShadow({ mode: "open" });

    const style = document.createElement("style");
    style.textContent = toastCss;
    this.root.appendChild(style);
  }

  connectedCallback(): void {
    this.toastContainer = document.createElement("div");
    this.toastContainer.className = "toaster";
    this.root.appendChild(this.toastContainer);
  }

  disconnectedCallback(): void {
    if (this.toastContainer) {
      this.toastContainer = null;
    }
  }

  public async toast(type: ToastType, messages: string): Promise<void> {
    console.log("🥪", type, messages);
    if (!this.toastContainer) {
      return;
    }

    let colorClass = "";
    switch (type) {
      case "error":
        colorClass = "red";
        break;
      case "warning":
        colorClass = "pumpkin";
        break;
      case "info":
        colorClass = "blue";
        break;
      case "success":
        colorClass = "green";
        break;
    }

    const node = document.createElement("output");
    node.setAttribute("role", "status");
    node.style.backgroundColor = `var(--pico-color-${colorClass}-600)`;
    node.style.color = `var(--pico-color-light)`;
    node.innerHTML = `
<p>${messages}</p>
<button class="close" title="close" style="color:inherit;">
  <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" stroke-width="3" stroke="currentColor" fill="none" stroke-linecap="round" stroke-linejoin="round" class="icon-close"><path d="M18 6l-12 12"></path><path d="M6 6l12 12"></path></svg>
</button>
`;

    const btn = getRequireElement("button", node);
    btn.addEventListener("click", () => {
      try {
        this.toastContainer?.removeChild(node);
      } catch (_) {
        // swallow exception:
        // the node it node present, that's OK for us
      }
    });

    if (this.toastContainer.children.length) {
      // FLIP https://aerotwist.com/blog/flip-your-animations/
      const first = this.toastContainer.offsetHeight;
      this.toastContainer.appendChild(node);
      const last = this.toastContainer.offsetHeight;
      const invert = last - first;
      this.toastContainer.animate(
        [{ transform: `translateY(${invert}px)` }, { transform: "translateY(0)" }],
        {
          duration: 150,
          easing: "ease-out",
        }
      );
    } else {
      this.toastContainer.appendChild(node);
    }

    // wait the animation ends
    const allFinished = node.getAnimations().map((animation) => animation.finished);
    await Promise.allSettled(allFinished);

    // remove the element
    try {
      this.toastContainer.removeChild(node);
    } catch (_err) {
      // swallow error
    }
  }
}

// Register the custom element
if (!customElements.get("toboggan-toast")) {
  customElements.define("toboggan-toast", TobogganToastElement);
}

declare global {
  interface HTMLElementTagNameMap {
    "toboggan-toast": TobogganToastElement;
  }
}
