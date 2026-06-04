/**
 * Executable JSR entrypoint for the `nr` run-script command.
 *
 * @module
 */

import { runInvocation } from "./shared.ts";

if (import.meta.main) {
  await runInvocation("nr");
}
