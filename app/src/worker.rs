//! Hintergrund-Analyse (ANA-01): arbeitet die Warteschlange nicht analysierter Tracks ab.

use crate::state::AppState;
use dw_core::ANALYSIS_VERSION;
use std::collections::HashSet;
use std::time::Duration;

pub fn spawn(state: AppState) {
    std::thread::Builder::new()
        .name("deckwerk-analyse".into())
        .spawn(move || {
            // Tracks, die sich nicht analysieren ließen, werden in diesem Lauf nicht erneut versucht.
            let mut failed: HashSet<i64> = HashSet::new();
            loop {
                let pending = state.lib.lock().unwrap().pending_analysis(ANALYSIS_VERSION).unwrap_or_default();
                let next = pending.into_iter().map(|(id, _)| id).find(|id| !failed.contains(id));
                match next {
                    Some(id) => {
                        if state.analyze_track(id).is_err() {
                            failed.insert(id);
                        }
                    }
                    None => {
                        // Warten, bis ein Import weckt.
                        let (lock, cvar) = &state.work;
                        let mut flag = lock.lock().unwrap();
                        while !*flag {
                            flag = cvar.wait_timeout(flag, Duration::from_secs(30)).unwrap().0;
                        }
                        *flag = false;
                    }
                }
            }
        })
        .expect("Analyse-Thread konnte nicht gestartet werden");
}
