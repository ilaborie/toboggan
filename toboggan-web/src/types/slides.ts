import { Content } from "./contents";

/**
 * Slide identifier - u8 value starting from 0
 */
export type SlideId = number;



/**
 * Slide structure
 */
export type Slide = {
    kind: "Cover" | "Part" | "Standard";
    style: string[];
    title: Content;
    body: Content;
    notes: Content;
};

export type SlidesResponse = {
    slides: Slide[];
};