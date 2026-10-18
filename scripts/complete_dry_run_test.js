/**
 * Complete Dry Run Test Suite for `wasmc` Compiler Toolchain
 * 
 * Executes full end-to-end verification:
 * 1. CLI Compilation (`wasmc build`)
 * 2. WebAssembly Core 2.0 Binary Validation & In-Memory Execution (Node.js V8)
 * 3. Front-End Validation (`wasmc check`)
 * 4. HTTP/2 Compiler REST Daemon (`wasmc daemon`) API Verification
 */

import { execSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';

const userProfile = process.env.USERPROFILE || 'C:\\Users\\ASUS';
const localAppData = process.env.LOCALAPPDATA || path.join(userProfile, 'AppData', 'Local');
const mingwBin = path.join(localAppData, 'Microsoft', 'WinGet', 'Packages', 'MartinStorsjo.LLVM-MinGW.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe', 'llvm-mingw-20260908-ucrt-x86_64', 'bin');
const cargoBin = path.join(userProfile, '.cargo', 'bin');

const defaultTargetBin = path.resolve(path.join(process.cwd(), 'target', 'debug', 'wasmc.exe'));
const tempTargetBin = path.resolve(userProfile, 'AppData/Local/Temp/wasmc_target/debug/wasmc.exe');
const WASMC_BIN = process.env.WASMC_BIN || (fs.existsSync(defaultTargetBin) ? defaultTargetBin : tempTargetBin);
let totalTests = 0;
let passedTests = 0;

function assert(condition, message) {
  totalTests++;
  if (!condition) {
    console.error(`  FAIL: ${message}`);
    throw new Error(`Assertion failed: ${message}`);
  }
  console.log(`  PASS: ${message}`);
  passedTests++;
}

async function runCommand(cmd) {
  return execSync(cmd, { encoding: 'utf-8', env: { ...process.env, PATH: `${cargoBin};${mingwBin};${process.env.PATH}` } });
}

async function httpRequest(options, data) {
  return new Promise((resolve, reject) => {
    const req = http.request(options, (res) => {
      let body = '';
      res.on('data', (chunk) => body += chunk);
      res.on('end', () => resolve({ statusCode: res.statusCode, body: body ? JSON.parse(body) : null }));
    });
    req.on('error', reject);
    if (data) req.write(JSON.stringify(data));
    req.end();
  });
}

async function main() {
  console.log('================================================================');
  console.log('       STARTING COMPLETE DRY RUN TEST FOR WASMC TOOLCHAIN       ');
  console.log('================================================================\n');

  // STEP 1: Verify wasmc binary version & help
  console.log('--- Step 1: CLI Version and Subcommand Checks ---');
  const helpOutput = execSync(`"${WASMC_BIN}" --help`, { encoding: 'utf-8' });
  assert(helpOutput.includes('Direct-to-WebAssembly Systems Compiler'), 'wasmc --help displays correct header');
  assert(helpOutput.includes('build') && helpOutput.includes('check') && helpOutput.includes('daemon'), 'wasmc subcommands present');

  // STEP 2: Compile all examples
  console.log('\n--- Step 2: Direct-to-Wasm Binary Compilation ---');
  const examples = [
    { src: 'examples/arithmetic.src', wasm: 'examples/arithmetic.wasm' },
    { src: 'examples/control_flow.src', wasm: 'examples/control_flow.wasm' },
    { src: 'examples/functions.src', wasm: 'examples/functions.wasm' },
    { src: 'examples/hello.src', wasm: 'examples/hello.wasm' },
    { src: 'examples/memory.src', wasm: 'examples/memory.wasm' },
  ];

  for (const ex of examples) {
    const out = await runCommand(`"${WASMC_BIN}" build ${ex.src} -o ${ex.wasm} --emit-wat`);
    assert(fs.existsSync(ex.wasm), `Successfully emitted ${ex.wasm}`);
    const bytes = fs.readFileSync(ex.wasm);
    assert(bytes.subarray(0, 4).toString('utf-8') === '\0asm', `${ex.wasm} has valid \\0asm magic bytes`);
    assert(bytes.readUInt32LE(4) === 1, `${ex.wasm} has valid Wasm Version 1 header`);
  }

  // STEP 3: V8 Runtime Execution & Value Assertions
  console.log('\n--- Step 3: Node.js V8 WebAssembly Runtime Execution & Assertions ---');

  // 3.1 Arithmetic
  const arithBytes = fs.readFileSync('examples/arithmetic.wasm');
  const { instance: arithInst } = await WebAssembly.instantiate(arithBytes);
  assert(arithInst.exports.add(40, 2) === 42, 'arithmetic.wasm: add(40, 2) == 42');
  assert(arithInst.exports.compute(10, 20) === 205, 'arithmetic.wasm: compute(10, 20) == 205');
  assert(Math.abs(arithInst.exports.float_calc(2.0, 3.0) - 7.0) < 1e-6, 'arithmetic.wasm: float_calc(2.0, 3.0) == 7.0');

  // 3.2 Control Flow & Loops
  const cfBytes = fs.readFileSync('examples/control_flow.wasm');
  const { instance: cfInst } = await WebAssembly.instantiate(cfBytes);
  assert(cfInst.exports.sum_up_to(10) === 55, 'control_flow.wasm: sum_up_to(10) == 55 (1+2+...+10)');
  assert(cfInst.exports.count_even(10) === 5, 'control_flow.wasm: count_even(10) == 5 (0,2,4,6,8)');

  // 3.3 Functions & Recursion
  const fnBytes = fs.readFileSync('examples/functions.wasm');
  const { instance: fnInst } = await WebAssembly.instantiate(fnBytes);
  assert(fnInst.exports.sum_of_squares(3, 4) === 25, 'functions.wasm: sum_of_squares(3, 4) == 25 (3^2 + 4^2)');
  assert(fnInst.exports.factorial(5) === 120, 'functions.wasm: factorial(5) == 120 (5!)');

  // 3.4 Memory & Shadow Stack
  const memBytes = fs.readFileSync('examples/memory.wasm');
  const { instance: memInst } = await WebAssembly.instantiate(memBytes);
  assert(memInst.exports.stack_math(10, 20) === 72, 'memory.wasm: stack_math(10, 20) == 72 (address-of local)');

  // 3.5 Host Import & String Marshalling
  const helloBytes = fs.readFileSync('examples/hello.wasm');
  let hostCalled = false;
  let receivedPtr = 0;
  let receivedLen = 0;
  const { instance: helloInst } = await WebAssembly.instantiate(helloBytes, {
    env: {
      print: (ptr, len) => {
        hostCalled = true;
        receivedPtr = ptr;
        receivedLen = len;
      }
    }
  });
  const helloRet = helloInst.exports.main();
  assert(helloRet === 0, 'hello.wasm: main() returns 0');
  assert(hostCalled === true, 'hello.wasm: host print callback was invoked');
  assert(receivedPtr === 1408 && receivedLen === 19, 'hello.wasm: correct string ptr (1408) and length (19) passed');

  // STEP 4: Front-end Validation (check command)
  console.log('\n--- Step 4: wasmc check Valid and Invalid Verification ---');
  const checkValid = await runCommand(`"${WASMC_BIN}" check examples/arithmetic.src`);
  assert(checkValid.includes('check passed successfully'), 'wasmc check on valid source succeeds');

  let checkFailed = false;
  try {
    execSync(`"${WASMC_BIN}" check examples/broken_syntax_test.src`, { stdio: 'pipe' });
  } catch (err) {
    checkFailed = true;
  }
  assert(checkFailed === true, 'wasmc check on non-existent or invalid source returns non-zero code');

  // STEP 5: Compiler Daemon REST API Verification
  console.log('\n--- Step 5: Compiler Daemon HTTP/2 REST API Verification ---');
  const daemonPort = 8089;
  const daemonProc = spawn(WASMC_BIN, ['daemon', '--host', '127.0.0.1', '--port', daemonPort.toString()]);
  
  // Give daemon 1 second to bind
  await new Promise((r) => setTimeout(r, 1000));

  try {
    // 5.1 Health Check
    const health = await httpRequest({
      hostname: '127.0.0.1',
      port: daemonPort,
      path: '/api/v1/health',
      method: 'GET',
    });
    assert(health.statusCode === 200 && health.body.status === 'UP', 'GET /api/v1/health returns HTTP 200 UP');

    // 5.2 Compile Endpoint
    const compilePayload = {
      project_id: 'dry_run_proj',
      sources: [
        {
          path: 'remote.src',
          content_utf8: 'export fn remote_mult(a: i32, b: i32) -> i32 { return a * b; }',
        }
      ],
      options: {
        target: 'wasm32-unknown-web',
        opt_level: 'O2',
      }
    };
    const compRes = await httpRequest({
      hostname: '127.0.0.1',
      port: daemonPort,
      path: '/api/v1/compile',
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
    }, compilePayload);

    assert(compRes.statusCode === 200, 'POST /api/v1/compile returns HTTP 200');
    assert(compRes.body.status === 'SUCCESS', 'POST /api/v1/compile status is SUCCESS');
    assert(Boolean(compRes.body.artifacts.wasm_base64), 'POST /api/v1/compile returned base64 wasm');
    assert(Boolean(compRes.body.artifacts.wasm_blake3_hash), 'POST /api/v1/compile returned BLAKE3 hash');

    // Test the compiled binary from daemon
    const daemonWasmBytes = Buffer.from(compRes.body.artifacts.wasm_base64, 'base64');
    const { instance: daemonInst } = await WebAssembly.instantiate(daemonWasmBytes);
    assert(daemonInst.exports.remote_mult(6, 7) === 42, 'Daemon compiled module: remote_mult(6, 7) == 42');

    // 5.3 Check Endpoint
    const checkRes = await httpRequest({
      hostname: '127.0.0.1',
      port: daemonPort,
      path: '/api/v1/check',
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
    }, compilePayload);
    assert(checkRes.statusCode === 200, 'POST /api/v1/check returns HTTP 200');
    assert(checkRes.body.status === 'SUCCESS', 'POST /api/v1/check status is SUCCESS');

    // 5.4 Cache Status Endpoint
    const cacheStatus = await httpRequest({
      hostname: '127.0.0.1',
      port: daemonPort,
      path: '/api/v1/cache/status',
      method: 'GET',
    });
    assert(cacheStatus.statusCode === 200, 'GET /api/v1/cache/status returns HTTP 200');

  } finally {
    daemonProc.kill();
  }

  console.log('\n================================================================');
  console.log(`COMPLETE DRY RUN TEST SUITE FINISHED: ${passedTests}/${totalTests} TESTS PASSED`);
  console.log('================================================================\n');
}

main().catch((err) => {
  console.error('\nDRY RUN FAILED WITH EXCEPTION:', err);
  process.exit(1);
});
