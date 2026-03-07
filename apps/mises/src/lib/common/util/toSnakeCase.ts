type UncapitalizeSnakeCase<S extends string> =
  S extends `${infer P1}${infer P2}`
    ? P2 extends Uncapitalize<P2> // Check if P2 starts with a lowercase letter (implying P1 was an uppercase letter starting a new word)
      ? `${Uncapitalize<P1>}${UncapitalizeSnakeCase<P2>}`
      : `${Uncapitalize<P1>}_${UncapitalizeSnakeCase<P2>}`
    : S;

type SnakeCase<T> = {
  [K in keyof T as K extends string ? UncapitalizeSnakeCase<K> : never]: T[K]
};

export function toSnakeCase<T>(object: T): SnakeCase<T> {
  return Object.fromEntries(Object.entries(object).map(([k, v]) => 
    [k.replace(/([A-Z])/g, '_$1').toLowerCase(), v]
  )) as SnakeCase<T>;
}