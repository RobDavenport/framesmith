export type LoadDecisionArgs = {
  requestedPath: string | null;
  loadedPath: string | null;
  inflightPath: string | null;
};

export function shouldStartLoad(args: LoadDecisionArgs): boolean {
  const requested = args.requestedPath;
  if (!requested) return false;
  if (requested === args.loadedPath) return false;
  if (requested === args.inflightPath) return false;
  return true;
}
