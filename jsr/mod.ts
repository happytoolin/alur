/**
 * JSR entrypoint for installing and running alur from Deno-compatible
 * TypeScript environments.
 *
 * @module
 */

export type { Invocation } from "./shared.ts";
export { ensureBinary, INVOCATIONS, runInvocation } from "./shared.ts";
