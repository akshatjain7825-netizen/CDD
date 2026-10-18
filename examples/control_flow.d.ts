// Type definitions for wasmc_module
export function instantiate(bytes: BufferSource | WebAssembly.Module, imports?: WebAssembly.Imports): Promise<typeof exports>;

export namespace exports {
  export const memory: WebAssembly.Memory;
  export function count_even(arg0: number): number;
  export function sum_up_to(arg0: number): number;
}
