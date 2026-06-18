/**
 * Executable JSR entrypoint for the `nex` package execution command.
 *
 * @module
 */

import { runInvocation } from "./shared.ts";

if (import.meta.main) {
  await runInvocation("nex");
}
