//! Independent buyer counting. Related wallets must not be counted as
//! independent participants. Verified links (explicit evidence) and weak links
//! (timing correlation only) are kept separate and both are surfaced.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IndependentBuyers {
    pub raw_wallets: usize,
    /// Wallets collapsed by verified links only. Upper bound on independence.
    pub max_independent: usize,
    /// Wallets collapsed by verified AND weak links. Conservative lower bound.
    pub min_independent: usize,
    /// Groups joined only by weak evidence, so the UI can say "possibly related".
    pub weakly_linked_groups: Vec<Vec<String>>,
}

struct Uf {
    parent: BTreeMap<String, String>,
}

impl Uf {
    fn new(items: &BTreeSet<String>) -> Self {
        Self {
            parent: items.iter().map(|i| (i.clone(), i.clone())).collect(),
        }
    }
    fn find(&mut self, x: &str) -> String {
        let p = self.parent.get(x).cloned().unwrap_or_else(|| x.to_string());
        if p == x {
            return p;
        }
        let r = self.find(&p);
        self.parent.insert(x.to_string(), r.clone());
        r
    }
    fn union(&mut self, a: &str, b: &str) {
        if !self.parent.contains_key(a) || !self.parent.contains_key(b) {
            return;
        }
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent.insert(ra, rb);
        }
    }
    fn groups(&mut self) -> BTreeMap<String, Vec<String>> {
        let keys: Vec<String> = self.parent.keys().cloned().collect();
        let mut g: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for k in keys {
            let r = self.find(&k);
            g.entry(r).or_default().push(k);
        }
        g
    }
}

pub fn count(
    buyers: &[String],
    verified: &[(String, String)],
    weak: &[(String, String)],
) -> IndependentBuyers {
    let set: BTreeSet<String> = buyers.iter().cloned().collect();
    let mut v = Uf::new(&set);
    for (a, b) in verified {
        v.union(a, b);
    }
    let max_independent = v.groups().len();
    let mut both = Uf::new(&set);
    for (a, b) in verified.iter().chain(weak.iter()) {
        both.union(a, b);
    }
    let both_groups = both.groups();
    let min_independent = both_groups.len();
    // Report groups that exist only because of weak links.
    let verified_groups = v.groups();
    let verified_root: BTreeMap<String, String> = verified_groups
        .iter()
        .flat_map(|(r, m)| m.iter().map(move |x| (x.clone(), r.clone())))
        .collect();
    let mut weakly_linked_groups = vec![];
    for members in both_groups.values() {
        let roots: BTreeSet<_> = members
            .iter()
            .filter_map(|m| verified_root.get(m))
            .collect();
        if roots.len() > 1 {
            weakly_linked_groups.push(members.clone());
        }
    }
    IndependentBuyers {
        raw_wallets: set.len(),
        max_independent,
        min_independent,
        weakly_linked_groups,
    }
}

/// A buy of a token by a wallet at a time, used to find timing correlation.
#[derive(Debug, Clone)]
pub struct Buy {
    pub wallet: String,
    pub token: String,
    pub at: DateTime<Utc>,
}

/// Weak links: wallet pairs that bought at least `min_shared` of the same
/// tokens within `window` of each other. Timing alone is weak evidence.
pub fn detect_weak_links(
    buys: &[Buy],
    window: Duration,
    min_shared: usize,
) -> Vec<(String, String)> {
    let mut by_token: BTreeMap<&str, Vec<&Buy>> = BTreeMap::new();
    for b in buys {
        by_token.entry(&b.token).or_default().push(b);
    }
    let mut shared: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for (token, v) in by_token {
        for (i, a) in v.iter().enumerate() {
            for b in &v[i + 1..] {
                if a.wallet != b.wallet && (a.at - b.at).abs() <= window {
                    let key = if a.wallet < b.wallet {
                        (a.wallet.clone(), b.wallet.clone())
                    } else {
                        (b.wallet.clone(), a.wallet.clone())
                    };
                    shared.entry(key).or_default().insert(token.to_string());
                }
            }
        }
    }
    shared
        .into_iter()
        .filter(|(_, t)| t.len() >= min_shared)
        .map(|(k, _)| k)
        .collect()
}
