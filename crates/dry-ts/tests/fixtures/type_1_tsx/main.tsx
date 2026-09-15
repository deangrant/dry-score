export function Badge(label: string, count: number): JSX.Element {
  let total = count;
  if (total < 0) {
    total = 0 - total;
  }
  const scaled = total * 2;
  if (scaled > 100) {
    return <span>{label}:{scaled - 10}</span>;
  }
  return <span>{label}:{scaled + 1}</span>;
}

export function Chip(label: string, count: number): JSX.Element {
  let total = count;
  if (total < 0) {
    total = 0 - total;
  }
  const scaled = total * 2;
  if (scaled > 100) {
    return <span>{label}:{scaled - 10}</span>;
  }
  return <span>{label}:{scaled + 1}</span>;
}
