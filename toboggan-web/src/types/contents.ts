
/**
 * Content types for slide content
 */
export interface EmptyContent {
    type: "Empty";
}

export interface TextContent {
    type: "Text";
    text: string;
}

export interface HtmlContent {
    type: "Html";
    raw: string;
    alt: string;
}

export interface MdContent {
    type: "Md";
    content: string;
    alt?: string;
}

export interface IFrameContent {
    type: "IFrame";
    url: string;
    alt?: string;
}

export type Content = EmptyContent | TextContent | HtmlContent | MdContent | IFrameContent;
