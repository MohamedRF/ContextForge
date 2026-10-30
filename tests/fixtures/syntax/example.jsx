import { water } from './garden';
export class Plant {
    grow() { water(); }
}
export const sprout = () => <Plant />;
export function seed() { return sprout(); }
