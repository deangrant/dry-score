export function nearLeft(value: number, weight: number): number {
  let acc = value;
  if (acc < 0) {
    acc = 0 - acc;
  }
  const product = acc * weight;
  if (product > 100) {
    return product - 10;
  }
  return product + 1;
}

export function nearRight(value: number, weight: number): number {
  let acc = value;
  if (acc < 0) {
    acc = 0 - acc;
  }
  const product = acc * weight;
  if (product > 100) {
    return product - 10;
  }
  const bonus = 2;
  return product + bonus;
}
