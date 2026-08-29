import { readFile, readdir } from 'node:fs/promises';
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
const emptyPages = [];
for (const path of files) {
  const source = await readFile(path, 'utf8');
  const body = source.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, '').trim();
  if (!body) {
    emptyPages.push(relative(docsRoot, path));
  }
}

if (missingEnglish.length || orphanedEnglish.length || emptyPages.length) {
  if (missingEnglish.length) {
    console.error(`Missing English mirrors:\n${missingEnglish.join('\n')}`);
  }
  if (orphanedEnglish.length) {
    console.error(`English pages without Chinese mirrors:\n${orphanedEnglish.join('\n')}`);
  }
  if (emptyPages.length) {
    console.error(`Empty Markdown pages:\n${emptyPages.join('\n')}`);
  }
  process.exitCode = 1;
} else {
  console.log(`Locale parity and non-empty content verified for ${chinesePages.length} page pairs.`);
}
