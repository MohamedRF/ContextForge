import { water } from './garden';
export interface Grow { grow(): void; }
export class Plant implements Grow {
    private count: number = 0;
    public grow(): void { water(); }
}
export type Name = string;
export enum Color { Green }
export const sprout = (name: Name) => new Plant();
namespace Garden { export function seed() {} }
