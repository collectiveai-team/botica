"use client";

export function searchUrl(query) {
  const debug = process.env.NODE_ENV !== "production";
  return `${process.env.SEARCH_API_URL}?q=${query}&key=${process.env.SEARCH_API_KEY}&debug=${debug}`;
}
