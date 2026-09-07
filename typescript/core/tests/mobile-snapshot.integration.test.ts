import { test, expect } from 'bun:test';
import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { createConnection, createServer } from 'node:net';
import { version } from '../package.json';
import { mobile, setServerAddr, shutdown } from '../src/index.js';

// Build the CLI and Android cdylib first, then run with ALLWRIGHT_TEST_MOBILE_SNAPSHOT=1.
// This exercises the real engine and plugin ABI, with simulated device XML and input.
const enabled = process.env.ALLWRIGHT_TEST_MOBILE_SNAPSHOT === '1';
const xml = '<hierarchy rotation="0"><node class="android.widget.Button" text="Save" clickable="true" enabled="true" bounds="[0,0][100,100]"/></hierarchy>';
test.skipIf(!enabled)('Android snapshots and reference actions cross the gRPC/plugin boundary', async () => {
  const dir = await mkdtemp(join(tmpdir(), 'allwright-mobile-snapshot-'));
  const xmlPath = join(dir, 'source.xml');
  const taps = join(dir, 'taps');
  await writeFile(xmlPath, xml);
  const port = await new Promise<number>((resolvePort) => {
    const socket = createServer();
    socket.listen(0, '127.0.0.1', () => {
      const port = (socket.address() as { port: number }).port;
      socket.close(() => resolvePort(port));
    });
  });
  const server = Bun.spawn([resolve('target/debug/allwright'), 'serve', '--listen-addr', `127.0.0.1:${port}`], {
    env: { ...process.env, ALLWRIGHT_HOME: dir, ALLWRIGHT_VERSION: version, ALLWRIGHT_AUTO_INSTALL: "1", ALLWRIGHT_ANDROID_ADB: resolve('typescript/core/tests/fixtures/snapshot-adb.py'), ALLWRIGHT_TEST_XML: xmlPath, ALLWRIGHT_TEST_TAPS: taps },
    stdout: 'ignore', stderr: 'inherit',
  });
  try {
    let ready = false;
    for (let i = 0; i < 100; i++) {
      ready = await new Promise<boolean>((done) => {
        const socket = createConnection({ host: '127.0.0.1', port });
        socket.once('connect', () => { socket.destroy(); done(true); });
        socket.once('error', () => { socket.destroy(); done(false); });
      });
      if (ready) break;
      await Bun.sleep(50);
    }
    expect(ready).toBe(true);
    setServerAddr(`127.0.0.1:${port}`);
    const device = await mobile.android.connect({ device: 'emulator-5554' });
    const app = device.app();
    const snapshot = JSON.parse(await app.accessibilitySnapshot({ mode: 'ai' }));
    const node = snapshot.documents[0].root.children[0];
    expect(node.role).toBe('button');
    expect(node.properties.xpath).toBe('/hierarchy/node[1]');
    const reference = app.locator(`ref=${node['aria-ref']}`);
    expect((await reference.count()).count).toBe(1);
    expect((await reference.textContent()).text).toBe('Save');
    await reference.click({ timeoutMs: 1 });
    expect(await readFile(taps, 'utf8')).toBe('50 50\n');
    expect(await app.accessibilitySnapshot({ format: 'yaml' })).toContain('"role": "button"');
    const other = (await mobile.android.connect({ device: 'emulator-5554' })).app();
    await expect(other.locator(`ref=${node['aria-ref']}`).count({ timeoutMs: 1 })).rejects.toThrow('foreign');
    await writeFile(xmlPath, xml.replace('Save', 'Delete'));
    await expect(reference.click({ timeoutMs: 1 })).rejects.toThrow('stale');
    expect(await readFile(taps, 'utf8')).toBe('50 50\n');
    const next = JSON.parse(await app.accessibilitySnapshot({ mode: 'ai' }));
    expect(next.documents[0].root.children[0]['aria-ref']).not.toBe(node['aria-ref']);
    expect(await readFile(xmlPath, 'utf8')).not.toContain('aria-ref');
  } finally {
    await shutdown();
    server.kill();
    await server.exited;
    await rm(dir, { recursive: true, force: true });
  }
}, 30_000);
