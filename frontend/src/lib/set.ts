export function union<T>(...sets: (Set<T> | Array<T>)[]): Set<T> {
  const result = new Set<T>();

  for (const set of sets) {
    for (const item of set) {
      result.add(item);
    }
  }

  return result;
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
