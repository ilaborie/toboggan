/**
 * Elements Module
 * Handles DOM element initialization and management
 */

import { ElementId } from "../constants/index.js";
import { getRequiredElement } from "../utils/dom.js";
import { TobogganNavigation } from "../components/navigation.js";

export interface PresentationElements {
  connectionStatus: HTMLElement;
  slideCounter: HTMLElement;
  durationDisplay: HTMLElement;
  errorDisplay: HTMLElement;
  appElement: HTMLElement;
  navigation?: TobogganNavigation;
}

export class ElementsModule {
  private elements: PresentationElements | null = null;

  /**
   * Initialize and return all required DOM elements
   */
  public initialize(): PresentationElements {
    if (this.elements) {
      return this.elements;
    }

    // Check if we have the new navigation web component
    const navigationElement = document.getElementById("navigation") as TobogganNavigation;
    
    if (navigationElement && navigationElement.tagName === "TOBOGGAN-NAVIGATION") {
      // New web component approach - create proxy elements that update the component
      this.elements = {
        connectionStatus: this.createNavigationProxy(navigationElement, "connection-status"),
        slideCounter: this.createNavigationProxy(navigationElement, "slide-counter"),
        durationDisplay: this.createNavigationProxy(navigationElement, "duration"),
        errorDisplay: getRequiredElement(ElementId.ERROR_DISPLAY),
        appElement: getRequiredElement(ElementId.APP),
        navigation: navigationElement,
      };
    } else {
      // Legacy approach - try to find individual elements
      this.elements = {
        connectionStatus: getRequiredElement(ElementId.CONNECTION_STATUS),
        slideCounter: getRequiredElement(ElementId.SLIDE_COUNTER),
        durationDisplay: getRequiredElement(ElementId.DURATION_DISPLAY),
        errorDisplay: getRequiredElement(ElementId.ERROR_DISPLAY),
        appElement: getRequiredElement(ElementId.APP),
      };
    }

    return this.elements;
  }

  /**
   * Create a proxy element that updates the navigation component
   */
  private createNavigationProxy(navigation: TobogganNavigation, attributeName: string): HTMLElement {
    const proxy = document.createElement("span");
    
    // Override textContent setter to update the navigation component
    Object.defineProperty(proxy, "textContent", {
      get: () => navigation.getAttribute(attributeName) || "",
      set: (value: string) => {
        navigation.setAttribute(attributeName, value);
      },
      enumerable: true,
      configurable: true,
    });

    return proxy;
  }

  /**
   * Get initialized elements (throws if not initialized)
   */
  public getElements(): PresentationElements {
    if (!this.elements) {
      throw new Error("Elements not initialized. Call initialize() first.");
    }
    return this.elements;
  }

  /**
   * Check if elements are initialized
   */
  public get isInitialized(): boolean {
    return this.elements !== null;
  }

  /**
   * Validate that all required elements exist in the DOM
   */
  public validate(): { valid: boolean; missing: string[] } {
    const missing: string[] = [];

    // Check if we have navigation web component
    const navigationElement = document.getElementById("navigation");
    if (navigationElement && navigationElement.tagName === "TOBOGGAN-NAVIGATION") {
      // For navigation component, only check for error display and app element
      const requiredIds = [ElementId.ERROR_DISPLAY, ElementId.APP];
      requiredIds.forEach((id) => {
        if (!document.getElementById(id)) {
          missing.push(id);
        }
      });
    } else {
      // Legacy validation - check all element IDs
      Object.values(ElementId).forEach((id) => {
        if (!document.getElementById(id)) {
          missing.push(id);
        }
      });
    }

    return {
      valid: missing.length === 0,
      missing,
    };
  }
}
