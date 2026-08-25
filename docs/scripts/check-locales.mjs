import { readdir } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const docsRoot = fileURLToPath(new URL('../src/content/docs', import.meta.url));

async function markdownFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...await markdownFiles(path));
    } else if (entry.isFile() && entry.name.endsWith('.md')) {
      files.push(path);
    }
  }
  return files;
}

const files = await markdownFiles(docsRoot);
const relativeFiles = new Set(files.map((path) => relative(docsRoot, path)));
const chinesePages = [...relativeFiles].filter((path) => !path.startsWith(`en\\`) && !path.startsWith('en/'));
const missingEnglish = chinesePages.filter((path) => !relativeFiles.has(join('en', path)));
const orphanedEnglish = [...relativeFiles]
  .filter((path) => path.startsWith(`en\\`) || path.startsWith('en/'))
  .filter((path) => !relativeFiles.has(path.replace(/^en[\\/]/, '')));

if (missingEnglish.length || orphanedEnglish.length) {
  if (missingEnglish.length) {
    console.error(`Missing English mirrors:\n${missingEnglish.join('\n')}`);
  }
  if (orphanedEnglish.length) {
    console.error(`English pages without Chinese mirrors:\n${orphanedEnglish.join('\n')}`);
  }
  process.exitCode = 1;
} else {
  console.log(`Locale parity verified for ${chinesePages.length} page pairs.`);
}
