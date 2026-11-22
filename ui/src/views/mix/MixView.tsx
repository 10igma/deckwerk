import { useUi } from '../../app/store';
import { DeckPanel, MiniDeck } from './DeckPanel';
import { LibraryPanel } from './LibraryPanel';
import { Mixer } from './Mixer';
import { TransitionEditor } from './TransitionEditor';
import { WaveStage } from './WaveStage';
import './mix.css';

/**
 * Ansicht „Mixen" (8.2): Wellenformen oben, darunter Deck A · Mixer · Deck B, unten die Bibliothek.
 * Ist die Bibliothek aufgezogen, schrumpfen die Decks auf je eine Zeile.
 */
export function MixView() {
  const big = useUi((s) => s.libraryBig);
  const editing = useUi((s) => s.editor !== null);
  return (
    <div className="mix" data-lib={big ? 'big' : 'normal'}>
      <WaveStage />
      {big ? (
        <div className="mix__mini">
          <MiniDeck deck="a" side="left" />
          <MiniDeck deck="b" side="right" />
        </div>
      ) : (
        <div className="mix__desk">
          <DeckPanel deck="a" side="left" />
          <Mixer />
          <DeckPanel deck="b" side="right" />
        </div>
      )}
      {/* Der Editor für Übergänge nimmt, solange er offen ist, den Platz der Bibliothek ein. */}
      {editing ? <TransitionEditor /> : <LibraryPanel />}
    </div>
  );
}
