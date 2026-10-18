// Type definitions for wasmc_module
export function instantiate(bytes: BufferSource | WebAssembly.Module, imports?: WebAssembly.Imports): Promise<typeof exports>;

export namespace exports {
  export const memory: WebAssembly.Memory;
  export function bubble_sort_count(arg0: number): number;
  export function reverse_bits(arg0: number): number;
  export function swap(arg0: number, arg1: number): void;
  export function clamp(arg0: number, arg1: number, arg2: number): number;
  export function is_power_of_two(arg0: number): number;
  export function digit_sum(arg0: number): number;
  export function lcm(arg0: number, arg1: number): number;
  export function count_primes(arg0: number): number;
  export function gcd(arg0: number, arg1: number): number;
  export function pow_int(arg0: number, arg1: number): number;
  export function collatz_steps(arg0: number): number;
  export function isqrt(arg0: number): number;
  export function popcount(arg0: number): number;
  export function is_prime(arg0: number): number;
}
