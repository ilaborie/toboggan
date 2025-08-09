/**
 * Content types for slide content
 */
export type EmptyContent = {
  type: "Empty";
};

export type TextContent = {
  type: "Text";
  text: string;
};

export type HtmlContent = {
  type: "Html";
  raw: string;
  alt: string;
};

export type MdContent = {
  type: "Md";
  content: string;
  alt?: string;
};

export type IFrameContent = {
  type: "IFrame";
  url: string;
  alt?: string;
};

export type Content = EmptyContent | TextContent | HtmlContent | MdContent | IFrameContent;
