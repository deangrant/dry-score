// Unique bodies so dry-ts dogfood stays at findings=0.

export function accumulateScores(values: number[]): number {
  let sum = 0;
  for (const value of values) {
    if (value > 0) {
      sum += value;
    }
  }
  return sum;
}
