export interface CachedChroma {
  id: number;
  name: string;
  colors: string[];
  skinChromaPath: string;
  skin_file?: string; // Path to the chroma's skin_file file
}
