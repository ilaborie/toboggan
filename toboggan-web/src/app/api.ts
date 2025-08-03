/**
 * Slides API Service
 * Handles fetching and caching of slides data
 */

import type { Slide, SlideId, SlidesResponse, Talk } from "../types";

export class SlidesApiService {
  private readonly apiBaseUrl: string;

  constructor(apiBaseUrl: string) {
    this.apiBaseUrl = apiBaseUrl;
  }

  private async get<T>(path: string): Promise<T> {
    const response = await fetch(`${this.apiBaseUrl}/${path}`);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    const result = await response.json();
    return result as T;
  }

  public async getTalk(): Promise<Talk> {
    return await this.get("api/talk");
  }

  public async getSlides(): Promise<Slide[]> {
    const response = await this.get<SlidesResponse>(`api/slides`);
    return response.slides;
  }

  public async getSlide(slideId: SlideId): Promise<Slide> {
    return await this.get(`api/slides/${slideId}`);
  }
}
