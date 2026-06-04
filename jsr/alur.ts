/**
 * Executable JSR entrypoint for the `alur` command.
 *
 * @module
 */

import { runInvocation } from "./shared.ts";

if (import.meta.main) {
  await runInvocation("alur");
}
