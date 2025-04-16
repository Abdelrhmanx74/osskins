export interface CachedChroma {
  id: number;
  name: string;
  colors: string[];
  skinChromaPath: string;
  fantome?: string; // Path to the chroma's fantome file
}
