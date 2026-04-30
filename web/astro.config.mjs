import { defineConfig } from 'astro/config'

const isGitHubPagesBuild = process.env.GITHUB_ACTIONS === 'true'

export default defineConfig({
  site: 'https://blu3ph4ntom.github.io',
  base: isGitHubPagesBuild ? '/pack' : '/',
  trailingSlash: 'always'
})
