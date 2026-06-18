/**
 * Executable JSR entrypoint for the `npar` parallel shell command.
 *
 * @module
 */

import { runInvocation } from "./shared.ts";

if (import.meta.main) {
  await runInvocation("npar");
}
