export function withNamed(): number {
  function left(n: number): number {
    let acc = n;
    if (acc < 0) {
      acc = 0 - acc;
    }
    return acc + 1;
  }
  function right(n: number): number {
    let acc = n;
    if (acc < 0) {
      acc = 0 - acc;
    }
    return acc + 1;
  }
  return left(3) + right(4);
}
