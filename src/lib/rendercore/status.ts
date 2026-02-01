import type { ActorStatus } from "./types";

function toNullableError(e: ActorStatus["error"]): string | null {
  if (typeof e === "string") return e;
  return null;
}

// Merge status reads with an error that occurred during update().
//
// Key behavior: an update() throw should not be overwritten by a later getStatus()
// call in the same tick.
export function mergeActorStatus(
  prev: ActorStatus,
  updateError: string | null,
  next: ActorStatus
): ActorStatus {
  return {
    loading: next.loading ?? prev.loading ?? false,
    error: updateError ?? toNullableError(next.error) ?? null,
  };
}
