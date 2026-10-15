// Type definitions for wasmc_module
export function instantiate(bytes: BufferSource | WebAssembly.Module, imports?: WebAssembly.Imports): Promise<typeof exports>;

export namespace exports {
  export const memory: WebAssembly.Memory;
  export function factorial(arg0: number): number;
  export function square(arg0: number): number;
  export function sum_of_squares(arg0: number, arg1: number): number;
}
