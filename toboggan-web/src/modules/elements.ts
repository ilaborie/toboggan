/**
 * Elements Module
 * Handles DOM element initialization and management
 */

import { ElementId } from "../constants/index.js";
import { getRequiredElement } from "../utils/dom.js";

export interface PresentationElements {
  connectionStatus: HTMLElement;
  slideCounter: HTMLElement;
  durationDisplay: HTMLElement;
  errorDisplay: HTMLElement;
  appElement: HTMLElement;
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

    this.elements = {
      connectionStatus: getRequiredElement(ElementId.CONNECTION_STATUS),
      slideCounter: getRequiredElement(ElementId.SLIDE_COUNTER),
      durationDisplay: getRequiredElement(ElementId.DURATION_DISPLAY),
      errorDisplay: getRequiredElement(ElementId.ERROR_DISPLAY),
      appElement: getRequiredElement(ElementId.APP),
    };

    return this.elements;
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

    Object.values(ElementId).forEach((id) => {
      if (!document.getElementById(id)) {
        missing.push(id);
      }
    });

    return {
      valid: missing.length === 0,
      missing,
    };
  }
}
