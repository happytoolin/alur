/**
 * Executable JSR entrypoint for the `nci` clean-install command.
 *
 * @module
 */

import { runInvocation } from "./shared.ts";

if (import.meta.main) {
  await runInvocation("nci");
}
