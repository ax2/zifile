import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const docsRoot = new URL('../src/content/docs/', import.meta.url);
const pages = {
  zh: await readFile(fileURLToPath(new URL('releases/stage-1.md', docsRoot)), 'utf8'),
  en: await readFile(fileURLToPath(new URL('en/releases/stage-1.md', docsRoot)), 'utf8'),
};

const requiredClaims = [
  ['zh', '纯 Rust RAR 只读 Beta'],
  ['zh', 'x64 和 ARM64 同时实现 5/5'],
  ['en', 'Pure-Rust read-only RAR beta'],
  ['en', 'produced 5/5 on both x64 and ARM64'],
];

const staleClaims = [
  ['en', 'RAR stayed outside 1.0 pending licensing'],
  ['en', 'four of five raw PE outputs match on each architecture'],
];

const failures = [];
for (const [locale, claim] of requiredClaims) {
  if (!pages[locale].includes(claim)) failures.push(`Missing current ${locale} claim: ${claim}`);
}
for (const [locale, claim] of staleClaims) {
  if (pages[locale].includes(claim)) failures.push(`Stale ${locale} claim returned: ${claim}`);
}

if (failures.length) {
  console.error(failures.join('\n'));
  process.exitCode = 1;
} else {
  console.log('Current RAR and Windows reproducibility claims verified.');
}
