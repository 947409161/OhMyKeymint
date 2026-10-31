export function afterPaint(): Promise<void> {
  return new Promise(resolve => {
    window.requestAnimationFrame(() => window.setTimeout(resolve, 0))
  })
}
