// scripts/run_wasm.js
// Universal runner to execute and inspect any compiled .wasm binary
import fs from 'node:fs';
import path from 'node:path';

const wasmFile = process.argv[2] || 'examples/sorting.wasm';
const filePath = path.resolve(wasmFile);

if (!fs.existsSync(filePath)) {
  console.error(`Error: File not found: ${filePath}`);
  process.exit(1);
}

const wasmBuffer = fs.readFileSync(filePath);

// Check magic bytes
const magic = wasmBuffer.subarray(0, 4).toString('hex');
if (magic !== '0061736d') {
  console.error(`Error: Invalid WebAssembly magic bytes (got: ${magic}, expected: 0061736d)`);
  process.exit(1);
}

console.log('='.repeat(60));
console.log(` WASMC WASM RUNNER: ${path.basename(filePath)} (${wasmBuffer.length} bytes)`);
console.log('='.repeat(60));

// Setup memory and host imports
const memory = new WebAssembly.Memory({ initial: 2 });
const importObject = {
  env: {
    memory,
    print: (ptr, len) => {
      const bytes = new Uint8Array(memory.buffer, ptr, len);
      const str = new TextDecoder().decode(bytes);
      console.log(`[Host Print]: "${str}"`);
    },
  },
};

WebAssembly.instantiate(wasmBuffer, importObject)
  .then(({ instance }) => {
    const exports = Object.keys(instance.exports);
    console.log(`\nExported Functions / Symbols (${exports.length}):`);
    exports.forEach(fn => console.log(`  • ${fn}`));

    console.log('\n--- Executing Sample Calls ---');

    if (instance.exports.bubble_sort) {
      const arr = [9, 3, 7, 1, 8, 2, 5];
      const mem = new Int32Array(memory.buffer, 100, arr.length);
      mem.set(arr);
      console.log(`Initial Array: [${Array.from(mem).join(', ')}]`);
      instance.exports.bubble_sort(100, arr.length);
      console.log(`Sorted Array:  [${Array.from(mem).join(', ')}]`);
    }

    if (instance.exports.bubble_sort_count) {
      console.log(`bubble_sort_count(5) = ${instance.exports.bubble_sort_count(5)} comparisons (in-place shadow-stack swaps)`);
    }

    if (instance.exports.collatz_steps) {
      [1, 6, 27].forEach(n => {
        console.log(`collatz_steps(${n}) = ${instance.exports.collatz_steps(n)}`);
      });
    }

    if (instance.exports.gcd) {
      console.log(`gcd(48, 18) = ${instance.exports.gcd(48, 18)}`);
    }

    if (instance.exports.factorial) {
      [1, 5, 7].forEach(n => {
        console.log(`factorial(${n}) = ${instance.exports.factorial(n)}`);
      });
    }

    if (instance.exports.add) {
      console.log(`add(40, 2) = ${instance.exports.add(40, 2)}`);
    }

    if (instance.exports.main) {
      const res = instance.exports.main();
      console.log(`main() exit code = ${res}`);
    }

    console.log('\nExecution completed successfully.');
    console.log('='.repeat(60));
  })
  .catch(err => {
    console.error('Execution Failed:', err);
    process.exit(1);
  });
