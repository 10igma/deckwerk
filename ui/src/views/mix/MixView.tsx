import { DeckPanel } from './DeckPanel';
import { LibraryPanel } from './LibraryPanel';
import { Mixer } from './Mixer';
import { WaveStage } from './WaveStage';
import './mix.css';

/** Ansicht „Mixen“ (8.2): Wellenformen oben, darunter Deck A · Mixer · Deck B, unten die Bibliothek. */
export function MixView() {
  return (
    <div className="mix">
      <WaveStage />
      <div className="mix__desk">
        <DeckPanel deck="a" side="left" />
        <Mixer />
        <DeckPanel deck="b" side="right" />
      </div>
      <LibraryPanel />
    </div>
  );
}
