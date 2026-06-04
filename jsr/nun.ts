/**
 * Executable JSR entrypoint for the `nun` uninstall command.
 *
 * @module
 */

import { runInvocation } from "./shared.ts";

if (import.meta.main) {
  await runInvocation("nun");
}
