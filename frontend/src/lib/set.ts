export function union<T>(
  setA: Set<T> | Array<T>,
  setB: Set<T> | Array<T>,
): Set<T> {
  return new Set([...setA, ...setB]);
}

export function intersection<T>(setA: Set<T>, setB: Set<T>): Set<T> {
  const intersectionSet = new Set<T>();
  const [smallerSet, largerSet] =
    setA.size < setB.size ? [setA, setB] : [setB, setA];

  for (const element of smallerSet) {
    if (largerSet.has(element)) {
      intersectionSet.add(element);
    }
  }

  return intersectionSet;
}

export function difference<T>(setA: Set<T>, setB: Set<T>): Set<T> {
  const differenceSet = new Set<T>();

  for (const element of setA) {
    if (!setB.has(element)) {
      differenceSet.add(element);
    }
  }

  return differenceSet;
}
