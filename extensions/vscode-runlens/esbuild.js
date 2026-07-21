// esbuild config — bundles extension.ts -> out/extension.js.
// Run via `node esbuild.js` from this directory.
const esbuild = require('esbuild');

(async () => {
    await esbuild.build({
        entryPoints: ['src/extension.ts'],
        bundle: true,
        outfile: 'out/extension.js',
        platform: 'node',
        target: 'node18',
        format: 'cjs',
        sourcemap: true,
        external: ['vscode'],
        logLevel: 'info',
    });
})().catch((err) => {
    console.error(err);
    process.exit(1);
});
