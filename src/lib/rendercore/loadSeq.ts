export interface LoadSeq {
  next(): number;
  isCurrent(n: number): boolean;
}

export function createLoadSeq(): LoadSeq {
  let current = 0;
  return {
    next() {
      current++;
      return current;
    },
    isCurrent(n) {
      return n === current;
    },
  };
}
