export function cacheUrl(): string {
  const url = process.env.CACHE_URL;
  if (!url) throw new Error("CACHE_URL required");
  return url;
}
