class Counter {
  scoreLeft(input: number, factor: number): number {
    let total = input;
    if (total < 0) {
      total = 0 - total;
    }
    const scaled = total * factor;
    if (scaled > 100) {
      return scaled - 10;
    }
    return scaled + 1;
  }

  scoreRight(input: number, factor: number): number {
    let total = input;
    if (total < 0) {
      total = 0 - total;
    }
    const scaled = total * factor;
    if (scaled > 100) {
      return scaled - 10;
    }
    return scaled + 1;
  }
}

export { Counter };
