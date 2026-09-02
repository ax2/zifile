import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const docsRoot = new URL('../src/content/docs/', import.meta.url);
const repositoryRoot = new URL('../../', import.meta.url);
const publicRelease = JSON.parse(
  await readFile(fileURLToPath(new URL('release/public-release.json', repositoryRoot)), 'utf8'),
);
const pages = {
  readme: await readFile(fileURLToPath(new URL('README.md', repositoryRoot)), 'utf8'),
  zh: await readFile(fileURLToPath(new URL('releases/stage-1.md', docsRoot)), 'utf8'),
  en: await readFile(fileURLToPath(new URL('en/releases/stage-1.md', docsRoot)), 'utf8'),
  privacyZh: await readFile(fileURLToPath(new URL('product/privacy.md', docsRoot)), 'utf8'),
  privacyEn: await readFile(fileURLToPath(new URL('en/product/privacy.md', docsRoot)), 'utf8'),
  gettingStartedZh: await readFile(
    fileURLToPath(new URL('guides/getting-started.md', docsRoot)),
    'utf8',
  ),
  gettingStartedEn: await readFile(
    fileURLToPath(new URL('en/guides/getting-started.md', docsRoot)),
    'utf8',
  ),
  stage4Zh: await readFile(fileURLToPath(new URL('releases/stage-4.md', docsRoot)), 'utf8'),
  stage4En: await readFile(fileURLToPath(new URL('en/releases/stage-4.md', docsRoot)), 'utf8'),
  roadmapZh: await readFile(fileURLToPath(new URL('product/roadmap.md', docsRoot)), 'utf8'),
  roadmapEn: await readFile(fileURLToPath(new URL('en/product/roadmap.md', docsRoot)), 'utf8'),
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

const expectedAssets = [
  `ZiFile-${publicRelease.version}.0-windows.msixbundle`,
  'zifile-windows-x64.exe',
  'zifile-windows-arm64.exe',
  'SHA256SUMS.txt',
];
if (publicRelease.schema_version !== 1) {
  failures.push(`Unsupported public release schema: ${publicRelease.schema_version}`);
}
if (publicRelease.tag !== `v${publicRelease.version}`) {
  failures.push(`Public release tag/version mismatch: ${publicRelease.tag} / ${publicRelease.version}`);
}
if (publicRelease.release_url !== `https://github.com/ax2/zifile/releases/tag/${publicRelease.tag}`) {
  failures.push(`Public release URL does not match ${publicRelease.tag}`);
}
if (
  typeof publicRelease.published_at !== 'string' ||
  !Number.isFinite(Date.parse(publicRelease.published_at))
) {
  failures.push(`Public release publication time is invalid: ${publicRelease.published_at}`);
}
const actualAssetNames = publicRelease.assets.map((asset) => asset.name);
if (JSON.stringify(actualAssetNames) !== JSON.stringify(expectedAssets)) {
  failures.push(`Public release assets must be exactly: ${expectedAssets.join(', ')}`);
}
for (const asset of publicRelease.assets) {
  if (asset.name !== asset.name.split(/[\\/]/).at(-1)) {
    failures.push(`Public release asset contains a path: ${asset.name}`);
  }
  if (!/^[a-f0-9]{64}$/.test(asset.sha256)) {
    failures.push(`Public release asset has an invalid SHA-256: ${asset.name}`);
  }
}

const currentReleaseClaims = [
  ['readme', `public [\`${publicRelease.tag}\` GitHub release](${publicRelease.release_url})`],
  ['gettingStartedZh', `[${publicRelease.tag} Release](${publicRelease.release_url})`],
  ['gettingStartedZh', `\`${expectedAssets[0]}\``],
  ['gettingStartedEn', `[${publicRelease.tag} Release](${publicRelease.release_url})`],
  ['gettingStartedEn', `\`${expectedAssets[0]}\``],
  ['stage4Zh', `\`${publicRelease.tag}\` 是当前面向 GitHub 的公开可用版本`],
  ['stage4En', `\`${publicRelease.tag}\` is the current usable public GitHub version`],
  ['roadmapZh', '| Stage 4（进行中） | 1.0 |'],
  ['roadmapEn', '| Stage 4 (active) | 1.0 |'],
];
for (const [page, claim] of currentReleaseClaims) {
  if (!pages[page].includes(claim)) failures.push(`Missing current ${page} claim: ${claim}`);
}
if (pages.roadmapZh.includes('Stage 3（进行中）')) {
  failures.push('Chinese roadmap still marks Stage 3 as active.');
}
if (pages.roadmapEn.includes('Stage 3 (active)')) {
  failures.push('English roadmap still marks Stage 3 as active.');
}

if (failures.length) {
  console.error(failures.join('\n'));
  process.exitCode = 1;
} else {
  console.log(
    `Current RAR, Windows reproducibility, local privacy, and ${publicRelease.tag} release claims verified.`,
  );
}
