import { execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const userProfile = process.env.USERPROFILE || 'C:\\Users\\ASUS';
const localAppData = process.env.LOCALAPPDATA || path.join(userProfile, 'AppData', 'Local');
const mingwBin = path.join(localAppData, 'Microsoft', 'WinGet', 'Packages', 'MartinStorsjo.LLVM-MinGW.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe', 'llvm-mingw-20260908-ucrt-x86_64', 'bin');
const cargoBin = path.join(userProfile, '.cargo', 'bin');

const defaultTargetBin = path.resolve(path.join(process.cwd(), 'target', 'debug', 'wasmc.exe'));
const WASMC_BIN = process.env.WASMC_BIN || defaultTargetBin;

const files = [
  'sorting.src',
  'arithmetic.src',
  'control_flow.src',
  'functions.src',
  'memory.src'
];

function countInstructions(watText) {
  let count = 0;
  const nonOpLines = /^\s*(\(|\)|module|type|memory|global|func|export|local|;;|$)/;
  for (const line of watText.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || nonOpLines.test(trimmed)) continue;
    count++;
  }
  return count;
}

console.log('File | WASM Bytes | WAT Instructions | Compile Time (ms)');
console.log('---|---|---|---');

const results = {};

for (const file of files) {
  const srcPath = path.join('examples', file);
  const wasmPath = path.join('examples', file.replace('.src', '.wasm'));
  const watPath = path.join('examples', file.replace('.src', '.wat'));

  // Run 5 iterations to measure average compilation latency
  const iterations = 5;
  let totalTime = 0;

  for (let i = 0; i < iterations; i++) {
    const start = performance.now();
    execSync(`"${WASMC_BIN}" build "${srcPath}" -o "${wasmPath}" --emit-wat`, {
      encoding: 'utf-8',
      env: { ...process.env, PATH: `${cargoBin};${mingwBin};${process.env.PATH}` }
    });
    const elapsed = performance.now() - start;
    totalTime += elapsed;
  }

  const avgTimeMs = (totalTime / iterations).toFixed(2);
  const wasmBytes = fs.statSync(wasmPath).size;
  const watText = fs.readFileSync(watPath, 'utf-8');
  const instCount = countInstructions(watText);

  results[file] = { wasmBytes, instCount, avgTimeMs };
  console.log(`${file} | ${wasmBytes} B | ${instCount} | ${avgTimeMs} ms`);
}

fs.writeFileSync('benchmark_results.json', JSON.stringify(results, null, 2));
