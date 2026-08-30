import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const docsRoot = new URL('../src/content/docs/', import.meta.url);
const pages = {
  zh: await readFile(fileURLToPath(new URL('releases/stage-1.md', docsRoot)), 'utf8'),
  en: await readFile(fileURLToPath(new URL('en/releases/stage-1.md', docsRoot)), 'utf8'),
  privacyZh: await readFile(fileURLToPath(new URL('product/privacy.md', docsRoot)), 'utf8'),
  privacyEn: await readFile(fileURLToPath(new URL('en/product/privacy.md', docsRoot)), 'utf8'),
};

const requiredClaims = [
  ['zh', '纯 Rust RAR 只读 Beta'],
  ['zh', 'x64 和 ARM64 同时实现 5/5'],
  ['en', 'Pure-Rust read-only RAR beta'],
  ['en', 'produced 5/5 on both x64 and ARM64'],
  ['privacyZh', '最多 8 个由 Worker 成功打开的本地压缩文件路径'],
  ['privacyZh', '这不是加密'],
  ['privacyEn', 'up to eight local archive paths that the Worker opened successfully'],
  ['privacyEn', 'not encryption'],
];

const staleClaims = [
  ['en', 'RAR stayed outside 1.0 pending licensing'],
  ['en', 'four of five raw PE outputs match on each architecture'],
  ['privacyZh', '仅在设备上的 `%LOCALAPPDATA%\\ZiFile\\settings.conf` 保存界面语言和主题'],
  ['privacyEn', 'stores only non-sensitive preferences such as interface language and theme'],
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
  console.log('Current RAR, Windows reproducibility, and local privacy claims verified.');
}
