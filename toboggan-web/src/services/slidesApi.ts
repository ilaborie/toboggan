/**
 * Slides API Service
 * Handles fetching and caching of slides data
 */

import type { Slide, SlideId, SlidesCache, SlidesResponse } from '../types.js';

export class SlidesApiService {
  private cache: SlidesCache | null = null;
  private readonly apiBaseUrl: string;

  constructor(apiBaseUrl: string) {
    this.apiBaseUrl = apiBaseUrl;
  }

  /**
   * Get a specific slide by ID
   * Fetches all slides if not cached
   */
  public async getSlide(slideId: SlideId): Promise<Slide> {
    if (!this.cache) {
      await this.fetchSlides();
    }

    const slide = this.cache!.slides[slideId.toString()];
    if (!slide) {
      throw new Error(`Slide ${slideId} not found`);
    }

    return slide;
  }

  /**
   * Get the total number of slides
   */
  public getTotalSlides(): number | null {
    return this.cache ? this.cache.orderedIds.length : null;
  }

  /**
   * Get the display number (1-based) for a slide ID
   */
  public getSlideDisplayNumber(slideId: SlideId): number {
    if (!this.cache) return slideId + 1;
    
    const index = this.cache.orderedIds.indexOf(slideId);
    return index >= 0 ? index + 1 : slideId + 1;
  }

  /**
   * Clear the slides cache
   */
  public clearCache(): void {
    this.cache = null;
  }

  /**
   * Fetch slides from the API and cache them
   */
  private async fetchSlides(): Promise<void> {
    const response = await fetch(`${this.apiBaseUrl}/api/slides`);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    const data: SlidesResponse = await response.json();
    
    // Create ordered slide IDs array
    const orderedIds = Object.keys(data.slides)
      .map(id => parseInt(id, 10))
      .sort((a, b) => a - b);
    
    // Cache the slides data
    this.cache = {
      slides: data.slides,
      orderedIds: orderedIds
    };
  }
}