/**
 * Elements Module
 * Handles DOM element initialization and management
 */

import { TobogganNavigationElement } from "./components/navigation.js";
import { TobogganToastElement } from "./components/toast.js";

const throwError = (main: string) => {
  throw new Error(main);
}

export interface PresentationElements {
  navigationElement: TobogganNavigationElement;
  appElement: HTMLElement;
  toastElement: TobogganToastElement;
}

/**
 * Initialize and return all required DOM elements
 */
export const loadPresentationElements = (): PresentationElements => {
  const appElement = document.querySelector("main") ?? throwError("missing <main> element");
  const navigation = document.querySelector("toboggan-navigation") ?? throwError("missing <toboggan-navigation> element");
  const toastElement = document.querySelector("toboggan-toast") ?? throwError("missing <toboggan-toast> element");

  return {
    navigationElement: navigation,
    appElement,
    toastElement,
  };
}
