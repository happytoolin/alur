/**
 * Executable JSR entrypoint for the `nlx` package execution command.
 *
 * @module
 */

import { runInvocation } from "./shared.ts";

if (import.meta.main) {
  await runInvocation("nlx");
}
