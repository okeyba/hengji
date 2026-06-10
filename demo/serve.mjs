import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const dir = dirname(fileURLToPath(import.meta.url));
const port = Number(process.env.PORT) || 4317;

createServer(async (_req, res) => {
  try {
    const html = await readFile(join(dir, 'index.html'));
    res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    res.end(html);
  } catch (e) {
    res.writeHead(500);
    res.end(String(e));
  }
}).listen(port, () => console.log(`demo landing on http://localhost:${port}`));
