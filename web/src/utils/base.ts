const externalUrlPattern = /^[a-zA-Z][a-zA-Z\d+\-.]*:/

export function withBase(path: string) {
  if (!path.startsWith('/')) return path
  if (path.startsWith('//') || externalUrlPattern.test(path)) return path

  const base = import.meta.env.BASE_URL ?? '/'
  if (base === '/') return path

  return `${base.replace(/\/$/, '')}${path}`
}
