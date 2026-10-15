// Type definitions for wasmc_module
export function instantiate(bytes: BufferSource | WebAssembly.Module, imports?: WebAssembly.Imports): Promise<typeof exports>;

export namespace exports {
  export const memory: WebAssembly.Memory;
  export function add(arg0: number, arg1: number): number;
  export function float_calc(arg0: number, arg1: number): number;
  export function compute(arg0: number, arg1: number): number;
}
