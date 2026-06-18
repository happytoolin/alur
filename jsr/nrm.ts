/**
 * Executable JSR entrypoint for the `nrm` uninstall command.
 *
 * @module
 */

import { runInvocation } from "./shared.ts";

if (import.meta.main) {
  await runInvocation("nrm");
}
