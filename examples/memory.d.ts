// Type definitions for wasmc_module
export function instantiate(bytes: BufferSource | WebAssembly.Module, imports?: WebAssembly.Imports): Promise<typeof exports>;

export namespace exports {
  export const memory: WebAssembly.Memory;
  export function stack_math(arg0: number, arg1: number): number;
}
