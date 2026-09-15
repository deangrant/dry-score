export function withClosures(): number {
  const left = (n: number): number => {
    let acc = n;
    if (acc < 0) {
      acc = 0 - acc;
    }
    return acc + 1;
  };
  const right = (n: number): number => {
    let acc = n;
    if (acc < 0) {
      acc = 0 - acc;
    }
    return acc + 1;
  };
  return left(3) + right(4);
}
