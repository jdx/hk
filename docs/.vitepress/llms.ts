// Generates docs/public/llms.txt, served at https://hk.jdx.dev/llms.txt.
// The page list comes from the VitePress sidebar and descriptions come from
// each page's lead paragraph so the index stays aligned with the documentation.

import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { sidebar, type SidebarItem } from './sidebar.ts'

const configDir = dirname(fileURLToPath(import.meta.url))
const docsDir = resolve(configDir, '..')
const outFile = resolve(docsDir, 'public/llms.txt')
const siteUrl = 'https://hk.jdx.dev'
const maxDescription = 200

function sourceFile(link: string): string {
  const relative = link.replace(/^\//, '')
  return resolve(
    docsDir,
    relative.endsWith('/') ? `${relative}index.md` : `${relative}.md`,
  )
}

function pageUrl(link: string): string {
  return link.endsWith('/')
    ? `${siteUrl}${link}`
    : `${siteUrl}${link}.html`
}

function plain(markdown: string): string {
  return markdown
    .replace(/!\[[^\]]*\]\([^)]*\)/g, '')
    .replace(/\[((?:[^[\]]|\[[^[\]]*\])*)\]\([^)]*\)/g, '$1')
    .replace(/`([^`]*)`/g, '$1')
    .replace(/[*_]{1,3}([^*_]+)[*_]{1,3}/g, '$1')
    .replace(/<[^>]+>/g, '')
    .replace(/\s+/g, ' ')
    .trim()
}

function description(file: string): string | undefined {
  let markdown: string
  try {
    markdown = readFileSync(file, 'utf8')
  } catch {
    return undefined
  }

  markdown = markdown.replace(/^---\n[\s\S]*?\n---\n/, '')

  let seenHeading = false
  const paragraph: string[] = []

  for (const raw of markdown.split('\n')) {
    const line = raw.trim()

    if (!seenHeading) {
      if (line.startsWith('# ')) seenHeading = true
      continue
    }
    if (paragraph.length === 0) {
      if (
        line === '' ||
        line.startsWith('#') ||
        line.startsWith(':::') ||
        line.startsWith('```') ||
        line.startsWith('<') ||
        line.startsWith('|') ||
        line.startsWith('- ') ||
        line.startsWith('* ') ||
        line.startsWith('**Choices:') ||
        line.startsWith('**Default:') ||
        line.startsWith('import ') ||
        line.startsWith('export ')
      ) {
        continue
      }
    } else if (
      line === '' ||
      line.startsWith('#') ||
      line.startsWith(':::')
    ) {
      break
    }
    paragraph.push(line.replace(/^>\s?/, ''))
  }

  const text = plain(paragraph.join(' ')).replace(/:$/, '.')
  if (!text) return undefined
  if (text.length <= maxDescription) return text

  const sentence = text.slice(0, maxDescription).lastIndexOf('. ')
  if (sentence > maxDescription / 2) return text.slice(0, sentence + 1)
  const word = text.lastIndexOf(' ', maxDescription)
  return `${text.slice(0, word > 0 ? word : maxDescription)}…`
}

interface Entry {
  text: string
  link: string
}

function flatten(items: SidebarItem[], entries: Entry[] = []): Entry[] {
  for (const item of items) {
    if (item.link?.startsWith('/')) {
      entries.push({ text: item.text, link: item.link })
    }
    if (item.items) flatten(item.items, entries)
  }
  return entries
}

const missing: string[] = []
const sections: string[] = []

for (const group of sidebar) {
  const entries = flatten(group.items ?? [])
  if (group.link?.startsWith('/')) {
    entries.unshift({ text: group.text, link: group.link })
  }
  if (entries.length === 0) continue

  const lines = entries.map(({ text, link }) => {
    const summary = description(sourceFile(link))
    if (!summary) missing.push(link)
    return summary
      ? `- [${text}](${pageUrl(link)}): ${summary}`
      : `- [${text}](${pageUrl(link)})`
  })
  sections.push(`## ${group.text}\n\n${lines.join('\n')}`)
}

const output = `# hk

> Run linters concurrently without letting overlapping fixes race

${sections.join('\n\n')}
`

writeFileSync(outFile, output)

const pages = output.split('\n').filter((line) => line.startsWith('- ')).length
console.log(`wrote ${outFile} (${pages} pages, ${output.length} bytes)`)
if (missing.length > 0) {
  console.log(`no lead paragraph found for ${missing.length} page(s):`)
  for (const link of missing) console.log(`  ${link}`)
}
