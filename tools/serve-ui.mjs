// The page (ui/) served for a browser, on the mock backend: `node tools/serve-ui.mjs [port]`,
// then http://localhost:5174/?mock. For working on a page without building the app.

import { createServer } from 'node:http';
import { readFile, readdir } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../ui/', import.meta.url));
const port = Number(process.argv[2] || process.env.PORT || 5174);
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.png': 'image/png', '.jpg': 'image/jpeg', '.json': 'application/json' };

// The texts in a language as `hub_texts` gives them (`hub_locales::texts`): every table of
// crates/hub-locales/locales read, the English text -> its translation.
const locales = fileURLToPath(new URL('../crates/hub-locales/locales/', import.meta.url));
const unquote = (s) => {
  s = s.trim();
  if (!(s.length >= 2 && s.startsWith('"') && s.endsWith('"'))) return s;
  return s.slice(1, -1).replace(/\\(.)/g, (_, c) => (c === 'n' ? '\n' : c === 't' ? '\t' : c));
};
async function texts(lang) {
  const out = {};
  for (const name of (await readdir(locales)).filter((n) => n.endsWith('.yml')).sort()) {
    let key = null;
    for (const line of (await readFile(join(locales, name), 'utf8')).split(/\r?\n/)) {
      if (!line.trim() || line.trimStart().startsWith('#') || line.startsWith('_version')) continue;
      if (line.startsWith('  ')) {
        const at = line.indexOf(':');
        if (key !== null && at > 0 && line.slice(2, at).trim().toLowerCase() === lang) out[key] = unquote(line.slice(at + 1));
      } else if (line.trimEnd().endsWith(':')) key = unquote(line.trimEnd().slice(0, -1));
    }
  }
  return out;
}

createServer(async (req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^([/\\])+/, '');
  const lang = path.replace(/\\/g, '/').match(/^mock-texts\/([a-z-]+)\.json$/)?.[1];
  if (lang) {
    res.writeHead(200, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' }).end(JSON.stringify(await texts(lang)));
    return;
  }
  const file = join(root, path || 'index.html');
  if (!file.startsWith(root)) {
    res.writeHead(403).end();
    return;
  }
  try {
    const body = await readFile(file);
    res.writeHead(200, { 'Content-Type': types[extname(file)] || 'application/octet-stream', 'Cache-Control': 'no-store' }).end(body);
  } catch {
    res.writeHead(404).end('not found');
  }
}).listen(port, '127.0.0.1', () => console.log(`ui on http://localhost:${port}/?mock`));
