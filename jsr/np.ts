/**
 * Executable JSR entrypoint for the `np` package-manager passthrough command.
 *
 * @module
 */

import { runInvocation } from "./shared.ts";

if (import.meta.main) {
  await runInvocation("np");
}
