export function computeSum(values: number[]): number {
  let total = 0;
  for (const value of values) {
    total += value;
  }
  if (total < 0) {
    return 0;
  }
  return total;
}

export function computeProduct(values: number[]): number {
  let total = 1;
  for (const value of values) {
    if (value === 0) {
      return 0;
    }
    total *= value;
  }
  return total;
}
