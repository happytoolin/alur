/**
 * Executable JSR entrypoint for the `ni` install command.
 *
 * @module
 */

import { runInvocation } from "./shared.ts";

if (import.meta.main) {
  await runInvocation("ni");
}
