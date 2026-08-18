export function cacheUrl(): string {
  const url = process.env.REDIS_URL;
  if (!url) throw new Error("REDIS_URL required");
  return url;
}
