import { cpSync, existsSync, readdirSync, rmSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const docsRoot = path.resolve(scriptDir, '..')
const distDir = path.join(docsRoot, 'dist')

if (!existsSync(distDir)) {
  throw new Error(`Missing Astro output: ${distDir}`)
}

const generatedEntries = ['_astro', 'docs', 'index.html', 'logo.png', 'favicon.svg']
for (const entry of generatedEntries) {
  const target = path.join(docsRoot, entry)
  if (existsSync(target)) {
    rmSync(target, { recursive: true, force: true })
  }
}

for (const entry of readdirSync(distDir)) {
  cpSync(path.join(distDir, entry), path.join(docsRoot, entry), { recursive: true })
}

