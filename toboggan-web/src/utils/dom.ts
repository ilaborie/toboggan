/**
 * DOM utility functions
 * Provides type-safe DOM element access
 */

import type { RequiredElement } from '../types.js';

/**
 * Get a required DOM element by ID
 * Throws an error if the element is not found
 */
export function getRequiredElement<T extends Element>(id: string): RequiredElement<T> {
  const element = document.getElementById(id) as T | null;
  if (!element) {
    throw new Error(`Required element with id '${id}' not found`);
  }
  return element as RequiredElement<T>;
}

/**
 * Escape HTML to prevent XSS attacks
 */
export function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

/**
 * Show an error message in the UI
 */
export function showError(errorDisplay: HTMLElement, message: string, duration: number = 5000): void {
  errorDisplay.textContent = message;
  errorDisplay.style.display = 'block';
  
  // Auto-hide error after specified duration
  setTimeout(() => {
    errorDisplay.style.display = 'none';
  }, duration);
}