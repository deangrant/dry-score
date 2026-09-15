export function alphaPath(value: number, weight: number): number {
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

export function betaPath(amount: number, multiplier: number): number {
  let running = amount;
  if (running < 0) {
    running = 0 - running;
  }
  const product = running * multiplier;
  if (product > 100) {
    return product - 10;
  }
  return product + 1;
}
