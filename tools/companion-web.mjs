// The Companion app as a web app, for the add-on to serve on the phone and tablet address
// (crates/hub-companion, `web`): the app's own web build (Expo, `expo export --platform web`)
// from its repository, each file gzipped into assets/companion-web/files/ under a short name of
// its own, and index.tsv saying which address each one is served at (the build's own paths run
// deep - assets/node_modules/@expo/vector-icons/... - past Windows' 260 characters in a deep
// checkout, and an ignore rule for node_modules/ would leave them out). Run it whenever the app
// changes; the add-on embeds what is there when it is built (crates/hub-companion/build.rs).
//
//   node tools/companion-web.mjs [path to Omsi-hub-companion-app]
//
// Without a path: OMSIHUB_COMPANION_APP, else the repository beside this one.

import { execSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync, constants } from 'node:zlib';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const app = resolve(process.argv[2] ?? process.env.OMSIHUB_COMPANION_APP ?? join(root, '..', 'Omsi-hub-companion-app'));
if (!existsSync(join(app, 'package.json'))) {
  console.error(`No Companion app at ${app}: give its folder.`);
  process.exit(1);
}
const out = join(root, 'assets', 'companion-web');
const tmp = mkdtempSync(join(tmpdir(), 'companion-web-'));

try {
  console.log(`Building the web app in ${app}…`);
  execSync(`npx expo export --platform web --output-dir "${tmp}"`, { cwd: app, stdio: 'inherit' });
  rmSync(out, { recursive: true, force: true });
  mkdirSync(join(out, 'files'), { recursive: true });
  const index = [];
  const walk = (d) => readdirSync(d).flatMap((n) => (statSync(join(d, n)).isDirectory() ? walk(join(d, n)) : [join(d, n)]));
  let raw = 0;
  let packed = 0;
  let count = 0;
  for (const file of walk(tmp)) {
    const rel = relative(tmp, file).split('\\').join('/');
    // (Expo's notes on the build; nothing a browser asks for)
    if (rel === 'metadata.json') continue;
    const bytes = readFileSync(file);
    const gz = gzipSync(bytes, { level: constants.Z_BEST_COMPRESSION });
    // (a short name: a few characters of the address's hash, then the file's own name)
    const base = rel.split('/').pop();
    const name = `${createHash('sha1').update(rel).digest('hex').slice(0, 8)}-${base.length > 60 ? base.slice(-60) : base}.gz`;
    writeFileSync(join(out, 'files', name), gz);
    index.push(`/${rel}\tfiles/${name}`);
    raw += bytes.length;
    packed += gz.length;
    count++;
  }
  writeFileSync(join(out, 'index.tsv'), index.sort().join('\n') + '\n');
  let commit = '';
  try {
    commit = execSync('git rev-parse --short HEAD', { cwd: app }).toString().trim();
    if (execSync('git status --porcelain', { cwd: app }).toString().trim()) commit += ' with changes not committed yet';
  } catch {
    // (not a repository: no commit to name)
  }
  writeFileSync(join(out, 'SOURCE.txt'), `The Companion app's web build (tools/companion-web.mjs)${commit ? `, from its commit ${commit}` : ''}.\n`);
  const mb = (n) => (n / 1048576).toFixed(1);
  console.log(`${count} files, ${mb(raw)} MB, gzipped ${mb(packed)} MB, in ${relative(root, out)}`);
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
